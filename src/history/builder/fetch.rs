use crate::core::client::{CacheMode, RetryConfig};
use crate::history::wire::{Events, MetaNode, QuoteBlock};

pub struct Fetched {
    pub ts: Vec<i64>,
    pub quote: QuoteBlock,
    pub adjclose: Vec<Option<f64>>,
    pub events: Option<Events>,
    pub meta: Option<MetaNode>,
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_chart(
    client: &crate::core::YfClient,
    symbol: &str,
    range: Option<crate::core::Range>,
    period: Option<(i64, i64)>,
    interval: crate::core::Interval,
    include_actions: bool,
    include_prepost: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Fetched, crate::core::YfError> {
    let mut url = client.base_chart().join(symbol)?;
    {
        let mut qp = url.query_pairs_mut();

        if let Some((p1, p2)) = period {
            if p1 >= p2 {
                return Err(crate::core::YfError::InvalidDates);
            }
            qp.append_pair("period1", &p1.to_string());
            qp.append_pair("period2", &p2.to_string());
        } else if let Some(r) = range {
            qp.append_pair("range", r.as_str());
        } else {
            return Err(crate::core::YfError::Data("no range or period set".into()));
        }

        qp.append_pair("interval", interval.as_str());
        if include_actions {
            qp.append_pair("events", "div|split|capitalGains");
        }
        qp.append_pair(
            "includePrePost",
            if include_prepost { "true" } else { "false" },
        );
    }

    if cache_mode == CacheMode::Use
        && let Some(body) = client.cache_get(&url).await
    {
        return decode_chart(&body);
    }

    let resp = client
        .send_with_retry(client.http().get(url.clone()), retry_override)
        .await?;
    if !resp.status().is_success() {
        return Err(crate::core::YfError::Status {
            status: resp.status().as_u16(),
            url: url.to_string(),
        });
    }

    let body = crate::core::net::get_text(resp, "history_chart", symbol, "json").await?;

    if cache_mode != CacheMode::Bypass {
        client.cache_put(&url, &body, None).await;
    }

    decode_chart(&body)
}

// NEW helper to keep fetch_chart compact
fn decode_chart(body: &str) -> Result<Fetched, crate::core::YfError> {
    use crate::history::wire::ChartEnvelope;
    let parsed: ChartEnvelope = serde_json::from_str(body)
        .map_err(|e| crate::core::YfError::Data(format!("json parse error: {e}")))?;

    let chart = parsed
        .chart
        .ok_or_else(|| crate::core::YfError::Data("missing chart".into()))?;

    if let Some(err) = chart.error {
        return Err(crate::core::YfError::Data(format!(
            "yahoo error: {} - {}",
            err.code, err.description
        )));
    }

    let mut results = chart
        .result
        .ok_or_else(|| crate::core::YfError::Data("missing result".into()))?;

    let r0 = results
        .pop()
        .ok_or_else(|| crate::core::YfError::Data("empty result".into()))?;

    let ts = r0.timestamp.unwrap_or_default();
    let quote = r0
        .indicators
        .quote
        .into_iter()
        .next()
        .ok_or_else(|| crate::core::YfError::Data("missing quote".into()))?;
    let adjclose = r0
        .indicators
        .adjclose
        .into_iter()
        .next()
        .map(|a| a.adjclose)
        .unwrap_or_default();

    Ok(Fetched {
        ts,
        quote,
        adjclose,
        events: r0.events,
        meta: r0.meta,
    })
}
