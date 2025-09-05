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
            qp.append_pair("range", crate::core::models::range_as_str(r));
        } else {
            return Err(crate::core::YfError::InvalidParams(
                "no range or period set".into(),
            ));
        }

        qp.append_pair("interval", crate::core::models::interval_as_str(interval));
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
    let envelope: crate::history::wire::ChartEnvelope =
        serde_json::from_str(body).map_err(crate::core::YfError::Json)?;

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

    Ok(Fetched {
        ts: first.timestamp.clone().unwrap_or_default(),
        quote: quote.clone(),
        adjclose,
        events: first.events.clone(),
        meta: first.meta.clone(),
    })
}
