use crate::core::{Interval, Range, YfClient, YfError};
use crate::history::wire::{ChartEnvelope, Events, MetaNode, QuoteBlock};

pub(crate) struct Fetched {
    pub ts: Vec<i64>,
    pub quote: QuoteBlock,
    pub adjclose: Vec<Option<f64>>,
    pub events: Option<Events>,
    pub meta: Option<MetaNode>,
}

pub(crate) async fn fetch_chart(
    client: &YfClient,
    symbol: &str,
    range: Option<Range>,
    period: Option<(i64, i64)>,
    interval: Interval,
    include_actions: bool,
    include_prepost: bool,
) -> Result<Fetched, YfError> {
    // Build URL
    let mut url = client.base_chart().join(symbol)?;
    {
        let mut qp = url.query_pairs_mut();

        if let Some((p1, p2)) = period {
            if p1 >= p2 {
                return Err(YfError::InvalidDates);
            }
            qp.append_pair("period1", &p1.to_string());
            qp.append_pair("period2", &p2.to_string());
        } else if let Some(r) = range {
            qp.append_pair("range", r.as_str());
        } else {
            return Err(YfError::Data("no range or period set".into()));
        }

        qp.append_pair("interval", interval.as_str());
        if include_actions {
            qp.append_pair("events", "div|split");
        }
        qp.append_pair(
            "includePrePost",
            if include_prepost { "true" } else { "false" },
        );
    }

    // Request
    let resp = client.http().get(url.clone()).send().await?;
    if !resp.status().is_success() {
        return Err(YfError::Status {
            status: resp.status().as_u16(),
            url: url.to_string(),
        });
    }

    // Parse
    let body = crate::core::net::get_text(resp, "history_chart", symbol, "json").await?;
    let parsed: ChartEnvelope =
        serde_json::from_str(&body).map_err(|e| YfError::Data(format!("json parse error: {e}")))?;

    let chart = parsed
        .chart
        .ok_or_else(|| YfError::Data("missing chart".into()))?;

    if let Some(err) = chart.error {
        return Err(YfError::Data(format!(
            "yahoo error: {} - {}",
            err.code, err.description
        )));
    }

    // Take ownership of the first result
    let mut results = chart
        .result
        .ok_or_else(|| YfError::Data("missing result".into()))?;

    let r0 = results
        .pop()
        .ok_or_else(|| YfError::Data("empty result".into()))?;

    let ts = r0.timestamp.unwrap_or_default();

    // Own the first quote block (and the first adjclose block, if any)
    let quotes = r0.indicators.quote;
    let quote = quotes
        .into_iter()
        .next()
        .ok_or_else(|| YfError::Data("missing quote".into()))?;

    let adjs = r0.indicators.adjclose;
    let adjclose = adjs
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
