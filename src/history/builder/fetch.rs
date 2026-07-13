use crate::core::wire::{JsonDecimal, WireValue};
use crate::core::{
    CallOptions, Interval, Range, YfError,
    client::{CacheEndpoint, SymbolEndpoint, normalize_symbol},
};
use crate::history::wire::{Events, MetaNode, QuoteBlock};
use chrono::Utc;

const SECONDS_PER_DAY: i64 = 86_400;
const MAX_RANGE_PROCESSING_SLACK_SECONDS: i64 = 5;
const MAX_RANGE_LONG_LOOKBACK_SECONDS: i64 = 99 * 365 * SECONDS_PER_DAY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartTimeQuery {
    Range(Range),
    Period { start: i64, end: i64 },
}

pub struct Fetched {
    pub ts: Vec<i64>,
    pub quote: QuoteBlock,
    pub adjclose: Vec<WireValue<JsonDecimal>>,
    pub events: Option<Events>,
    pub meta: Option<MetaNode>,
}

#[derive(Clone, Copy)]
pub struct ChartFetchRequest {
    pub range: Option<Range>,
    pub period: Option<(i64, i64)>,
    pub interval: Interval,
    pub include_actions: bool,
    pub include_prepost: bool,
}

impl ChartFetchRequest {
    fn time_query(self, now: i64) -> Result<ChartTimeQuery, YfError> {
        if let Some((start, end)) = self.period {
            return checked_period(start, end);
        }

        match self.range {
            Some(Range::Max) => {
                let (start, end) = max_range_period(self.interval, now)?;
                Ok(ChartTimeQuery::Period { start, end })
            }
            Some(range) => Ok(ChartTimeQuery::Range(range)),
            None => Err(YfError::InvalidParams("no range or period set".into())),
        }
    }
}

const fn checked_period(start: i64, end: i64) -> Result<ChartTimeQuery, YfError> {
    if start >= end {
        return Err(YfError::InvalidDates);
    }

    Ok(ChartTimeQuery::Period { start, end })
}

fn max_range_period(interval: Interval, end: i64) -> Result<(i64, i64), YfError> {
    // Yahoo silently coarsens `range=max` responses. Match Python yfinance by
    // translating the semantic maximum into an explicit retention-aware window.
    // Bucket the end by requested resolution so equivalent calls share a cache key.
    let end = timestamp_ceiling(end, interval.seconds().unwrap_or(SECONDS_PER_DAY))?;
    let lookback = match crate::core::models::interval_as_str(interval) {
        "1m" => 8 * SECONDS_PER_DAY,
        "2m" | "5m" | "15m" | "30m" | "90m" => 60 * SECONDS_PER_DAY,
        "1h" | "60m" => 730 * SECONDS_PER_DAY,
        _ => MAX_RANGE_LONG_LOOKBACK_SECONDS,
    };
    let start = end
        .checked_sub(lookback)
        .and_then(|start| start.checked_add(MAX_RANGE_PROCESSING_SLACK_SECONDS))
        .ok_or_else(|| {
            YfError::InvalidParams("maximum history window exceeds timestamp bounds".into())
        })?;

    Ok((start, end))
}

fn timestamp_ceiling(timestamp: i64, bucket: i64) -> Result<i64, YfError> {
    let remainder = timestamp.rem_euclid(bucket);
    if remainder == 0 {
        return Ok(timestamp);
    }

    timestamp.checked_add(bucket - remainder).ok_or_else(|| {
        YfError::InvalidParams("maximum history window exceeds timestamp bounds".into())
    })
}

pub async fn fetch_chart(
    client: &crate::core::YfClient,
    symbol: &str,
    request: ChartFetchRequest,
    options: &CallOptions,
) -> Result<Fetched, crate::core::YfError> {
    let symbol = normalize_symbol(symbol)?;
    let mut url = client.symbol_url(SymbolEndpoint::Chart, &symbol)?;
    let time_query = request.time_query(Utc::now().timestamp())?;
    {
        let mut qp = url.query_pairs_mut();

        match time_query {
            ChartTimeQuery::Period { start, end } => {
                qp.append_pair("period1", &start.to_string());
                qp.append_pair("period2", &end.to_string());
            }
            ChartTimeQuery::Range(range) => {
                qp.append_pair("range", crate::core::models::range_as_str(range));
            }
        }

        qp.append_pair(
            "interval",
            crate::core::models::interval_as_str(request.interval),
        );
        if request.include_actions {
            qp.append_pair("events", "div|split|capitalGains");
        }
        qp.append_pair(
            "includePrePost",
            if request.include_prepost {
                "true"
            } else {
                "false"
            },
        );
    }

    let body = crate::core::net::fetch_text_cached(
        client,
        &url,
        crate::core::net::CacheFetchConfig {
            cache_endpoint: CacheEndpoint::Chart,
            options,
            endpoint: "history_chart",
            fixture_key: &symbol,
            ext: "json",
            cache_validator: Some(validate_chart_body),
        },
    )
    .await?;

    let fetched = decode_chart(&body)?;
    if let Err(error) = validate_chart_granularity(request.interval, fetched.meta.as_ref()) {
        client.invalidate_cache_entry(&url);
        return Err(error);
    }
    Ok(fetched)
}

fn validate_chart_granularity(requested: Interval, meta: Option<&MetaNode>) -> Result<(), YfError> {
    let Some(actual) = meta
        .and_then(|meta| meta.data_granularity.as_deref())
        .map(str::trim)
        .filter(|actual| !actual.is_empty())
    else {
        return Ok(());
    };
    let expected = crate::core::models::interval_as_str(requested);
    let equivalent =
        actual == expected || matches!((expected, actual), ("1h", "60m") | ("60m", "1h"));

    if equivalent {
        Ok(())
    } else {
        Err(YfError::InvalidData(format!(
            "Yahoo returned {actual} chart granularity for requested {expected} interval"
        )))
    }
}

fn validate_chart_body(body: &str) -> Result<(), crate::core::YfError> {
    decode_chart(body).map(|_| ())
}

fn decode_chart(body: &str) -> Result<Fetched, crate::core::YfError> {
    let envelope: crate::history::wire::ChartEnvelope =
        serde_json::from_str(body).map_err(crate::core::YfError::json)?;

    let chart = envelope
        .chart
        .ok_or_else(|| crate::core::YfError::MissingData("missing chart".into()))?;

    if let Some(error) = chart.error {
        return Err(crate::core::YfError::Api(format!(
            "chart error: {} - {}",
            error.code, error.description
        )));
    }

    let result = chart
        .result
        .ok_or_else(|| crate::core::YfError::MissingData("missing result".into()))?;

    let first = result
        .first()
        .ok_or_else(|| crate::core::YfError::MissingData("empty result".into()))?;

    let quote = first
        .indicators
        .quote
        .first()
        .ok_or_else(|| crate::core::YfError::MissingData("missing quote".into()))?;
    let adjclose = first
        .indicators
        .adjclose
        .first()
        .map(|a| a.adjclose.clone())
        .unwrap_or_default();

    let ts = match first.timestamp.clone() {
        Some(ts) => ts,
        None if quote_block_is_empty(quote) && adjclose.is_empty() => Vec::new(),
        None => {
            return Err(crate::core::YfError::MissingData(
                "missing timestamps".into(),
            ));
        }
    };

    Ok(Fetched {
        ts,
        quote: quote.clone(),
        adjclose,
        events: first.events.clone(),
        meta: first.meta.clone(),
    })
}

const fn quote_block_is_empty(quote: &QuoteBlock) -> bool {
    quote.open.is_empty()
        && quote.high.is_empty()
        && quote.low.is_empty()
        && quote.close.is_empty()
        && quote.volume.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_012_345;

    #[test]
    fn max_range_uses_python_compatible_interval_windows() {
        for (interval, lookback) in [
            (Interval::I1m, 8 * SECONDS_PER_DAY),
            (Interval::I2m, 60 * SECONDS_PER_DAY),
            (Interval::I5m, 60 * SECONDS_PER_DAY),
            (Interval::I15m, 60 * SECONDS_PER_DAY),
            (Interval::I30m, 60 * SECONDS_PER_DAY),
            (Interval::I90m, 60 * SECONDS_PER_DAY),
            (Interval::I1h, 730 * SECONDS_PER_DAY),
            (Interval::D1, MAX_RANGE_LONG_LOOKBACK_SECONDS),
            (Interval::M1, MAX_RANGE_LONG_LOOKBACK_SECONDS),
        ] {
            let request = ChartFetchRequest {
                range: Some(Range::Max),
                period: None,
                interval,
                include_actions: false,
                include_prepost: false,
            };

            let end =
                timestamp_ceiling(NOW, interval.seconds().unwrap_or(SECONDS_PER_DAY)).unwrap();
            assert_eq!(
                request.time_query(NOW).unwrap(),
                ChartTimeQuery::Period {
                    start: end - lookback + MAX_RANGE_PROCESSING_SLACK_SECONDS,
                    end,
                }
            );
        }
    }

    #[test]
    fn max_range_query_is_stable_within_interval_bucket() {
        let request = ChartFetchRequest {
            range: Some(Range::Max),
            period: None,
            interval: Interval::D1,
            include_actions: false,
            include_prepost: false,
        };

        assert_eq!(
            request.time_query(NOW).unwrap(),
            request.time_query(NOW + 3_600).unwrap()
        );
    }

    #[test]
    fn max_range_timestamp_arithmetic_is_checked() {
        let request = ChartFetchRequest {
            range: Some(Range::Max),
            period: None,
            interval: Interval::D1,
            include_actions: false,
            include_prepost: false,
        };

        assert!(matches!(
            request.time_query(i64::MIN),
            Err(YfError::InvalidParams(_))
        ));
        assert!(matches!(
            request.time_query(i64::MAX),
            Err(YfError::InvalidParams(_))
        ));
    }

    #[test]
    fn explicit_period_takes_precedence_over_range() {
        let request = ChartFetchRequest {
            range: Some(Range::Max),
            period: Some((100, 200)),
            interval: Interval::D1,
            include_actions: false,
            include_prepost: false,
        };

        assert_eq!(
            request.time_query(NOW).unwrap(),
            ChartTimeQuery::Period {
                start: 100,
                end: 200,
            }
        );
    }

    #[test]
    fn non_max_range_stays_relative() {
        let request = ChartFetchRequest {
            range: Some(Range::M6),
            period: None,
            interval: Interval::D1,
            include_actions: false,
            include_prepost: false,
        };

        assert_eq!(
            request.time_query(NOW).unwrap(),
            ChartTimeQuery::Range(Range::M6)
        );
    }

    #[test]
    fn chart_granularity_must_match_requested_interval() {
        let matching = MetaNode {
            data_granularity: Some("1d".into()),
            ..MetaNode::default()
        };
        let mismatched = MetaNode {
            data_granularity: Some("3mo".into()),
            ..MetaNode::default()
        };
        let blank = MetaNode {
            data_granularity: Some("  ".into()),
            ..MetaNode::default()
        };
        let hourly_alias = MetaNode {
            data_granularity: Some("60m".into()),
            ..MetaNode::default()
        };

        assert!(validate_chart_granularity(Interval::D1, Some(&matching)).is_ok());
        assert!(matches!(
            validate_chart_granularity(Interval::D1, Some(&mismatched)),
            Err(YfError::InvalidData(message))
                if message.contains("3mo") && message.contains("1d")
        ));
        assert!(validate_chart_granularity(Interval::D1, None).is_ok());
        assert!(validate_chart_granularity(Interval::D1, Some(&blank)).is_ok());
        assert!(validate_chart_granularity(Interval::I1h, Some(&hourly_alias)).is_ok());
    }
}
