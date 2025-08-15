use crate::{YfClient, error::YfError, types::Candle};
use serde::Deserialize;

/// Common ranges Yahoo accepts for daily data.
#[derive(Debug, Clone, Copy)]
pub enum Range {
    D5,
    M1,
    M3,
    M6,
    Y1,
    Y2,
    Y5,
    Max,
}
impl Range {
    fn as_str(self) -> &'static str {
        match self {
            Range::D5 => "5d",
            Range::M1 => "1mo",
            Range::M3 => "3mo",
            Range::M6 => "6mo",
            Range::Y1 => "1y",
            Range::Y2 => "2y",
            Range::Y5 => "5y",
            Range::Max => "max",
        }
    }
}

/// Builder for history queries (room to add params later).
pub struct HistoryBuilder<'a> {
    client: &'a YfClient,
    symbol: String,
    // If `period` is set, we use period1/period2; otherwise we use `range`.
    range: Option<Range>,
    period: Option<(i64, i64)>, // (period1, period2) in seconds UTC
}

impl<'a> HistoryBuilder<'a> {
    pub fn new(client: &'a YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client,
            symbol: symbol.into(),
            range: Some(Range::M6),
            period: None,
        }
    }

    /// Use a predefined relative range (e.g., 6 months).
    /// Calling this clears any absolute period previously set.
    pub fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }

    /// Use absolute UTC dates. `start < end` is required.
    /// This takes precedence over any previously set `.range(...)`.
    pub fn between(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.range = None;
        self.period = Some((start.timestamp(), end.timestamp()));
        self
    }

    pub async fn fetch(self) -> Result<Vec<Candle>, YfError> {
        // Build URL: <base>/<symbol>?...(either range or period1/period2)...&interval=1d&events=div|split
        let mut url = self.client.base_chart().join(&self.symbol)?;
        {
            let mut qp = url.query_pairs_mut();

            if let Some((p1, p2)) = self.period {
                if p1 >= p2 {
                    return Err(YfError::InvalidDates);
                }
                qp.append_pair("period1", &p1.to_string());
                qp.append_pair("period2", &p2.to_string());
            } else if let Some(r) = self.range {
                qp.append_pair("range", r.as_str());
            } else {
                // Shouldn't happen, but be defensive.
                return Err(YfError::Data("no range or period set".into()));
            }

            qp.append_pair("interval", "1d");
            qp.append_pair("events", "div|split");
        }

        let resp = self.client.http().get(url.clone()).send().await?;
        if !resp.status().is_success() {
            return Err(YfError::Status {
                status: resp.status().as_u16(),
                url: url.to_string(),
            });
        }
        let body = resp.text().await?;
        let parsed: ChartEnvelope = serde_json::from_str(&body)
            .map_err(|e| YfError::Data(format!("json parse error: {e}")))?;

        let chart = parsed
            .chart
            .ok_or_else(|| YfError::Data("missing chart".into()))?;

        if let Some(err) = chart.error {
            return Err(YfError::Data(format!(
                "yahoo error: {} - {}",
                err.code, err.description
            )));
        }
        let mut out = Vec::new();

        let result = chart
            .result
            .ok_or_else(|| YfError::Data("missing result".into()))?;
        let r0 = result.first()
            .ok_or_else(|| YfError::Data("empty result".into()))?;

        let ts = r0.timestamp.as_deref().unwrap_or(&[]);
        let q = r0
            .indicators
            .quote.first()
            .ok_or_else(|| YfError::Data("missing quote".into()))?;

        // Defensive: lengths should align; skip rows that are None or out-of-bounds.
        for (i, &t) in ts.iter().enumerate() {
            let getter = |v: &Vec<Option<f64>>| v.get(i).and_then(|x| *x);
            let open = getter(&q.open);
            let high = getter(&q.high);
            let low = getter(&q.low);
            let close = getter(&q.close);
            if let (Some(open), Some(high), Some(low), Some(close)) = (open, high, low, close) {
                let volume = q.volume.get(i).and_then(|x| *x);
                out.push(Candle {
                    ts: t,
                    open,
                    high,
                    low,
                    close,
                    volume,
                });
            }
        }
        Ok(out)
    }
}

/* --- Internal response mapping (only fields we need) --- */

#[derive(Deserialize)]
struct ChartEnvelope {
    chart: Option<ChartNode>,
}

#[derive(Deserialize)]
struct ChartNode {
    result: Option<Vec<ChartResult>>,
    error: Option<ChartError>,
}
#[derive(Deserialize)]
struct ChartError {
    code: String,
    description: String,
}

#[derive(Deserialize)]
struct ChartResult {
    #[serde(default)]
    timestamp: Option<Vec<i64>>,
    indicators: Indicators,
}

#[derive(Deserialize)]
struct Indicators {
    #[serde(default)]
    quote: Vec<QuoteBlock>,
}

#[derive(Deserialize)]
struct QuoteBlock {
    #[serde(default)]
    open: Vec<Option<f64>>,
    #[serde(default)]
    high: Vec<Option<f64>>,
    #[serde(default)]
    low: Vec<Option<f64>>,
    #[serde(default)]
    close: Vec<Option<f64>>,
    #[serde(default)]
    volume: Vec<Option<u64>>,
}
