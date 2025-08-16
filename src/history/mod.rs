use crate::{YfClient, error::YfError};
use serde::Deserialize;

mod model;
pub use model::{Action, Candle, HistoryMeta, HistoryResponse};

#[derive(Debug, Clone, Copy)]
pub enum Range {
    D1,
    D5,
    M1,
    M3,
    M6,
    Y1,
    Y2,
    Y5,
    Y10,
    Ytd,
    Max,
}
impl Range {
    fn as_str(self) -> &'static str {
        match self {
            Range::D1 => "1d",
            Range::D5 => "5d",
            Range::M1 => "1mo",
            Range::M3 => "3mo",
            Range::M6 => "6mo",
            Range::Y1 => "1y",
            Range::Y2 => "2y",
            Range::Y5 => "5y",
            Range::Y10 => "10y",
            Range::Ytd => "ytd",
            Range::Max => "max",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Interval {
    I1m,
    I2m,
    I5m,
    I15m,
    I30m,
    I60m,
    I90m,
    I1h,
    D1,
    D5,
    W1,
    M1,
    M3,
}
impl Interval {
    fn as_str(self) -> &'static str {
        match self {
            Interval::I1m => "1m",
            Interval::I2m => "2m",
            Interval::I5m => "5m",
            Interval::I15m => "15m",
            Interval::I30m => "30m",
            Interval::I60m => "60m",
            Interval::I90m => "90m",
            Interval::I1h => "1h",
            Interval::D1 => "1d",
            Interval::D5 => "5d",
            Interval::W1 => "1wk",
            Interval::M1 => "1mo",
            Interval::M3 => "3mo",
        }
    }
}

pub struct HistoryBuilder<'a> {
    client: &'a YfClient,
    symbol: String,
    range: Option<Range>,
    period: Option<(i64, i64)>,
    interval: Interval,
    auto_adjust: bool,
    include_prepost: bool,
    include_actions: bool,
    keepna: bool,
}

impl<'a> HistoryBuilder<'a> {
    pub fn new(client: &'a YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client,
            symbol: symbol.into(),
            range: Some(Range::M6),
            period: None,
            interval: Interval::D1,
            auto_adjust: true,
            include_prepost: false,
            include_actions: true,
            keepna: false,
        }
    }

    pub fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }

    pub fn between(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.range = None;
        self.period = Some((start.timestamp(), end.timestamp()));
        self
    }

    pub fn interval(mut self, interval: Interval) -> Self {
        self.interval = interval;
        self
    }

    pub fn auto_adjust(mut self, yes: bool) -> Self {
        self.auto_adjust = yes;
        self
    }

    pub fn prepost(mut self, yes: bool) -> Self {
        self.include_prepost = yes;
        self
    }

    pub fn actions(mut self, yes: bool) -> Self {
        self.include_actions = yes;
        self
    }

    /// Keep rows where OHLC is partially/fully missing, inserting NaN values for missing fields.
    /// Default is false (drop rows with any missing OHLC).
    pub fn keepna(mut self, yes: bool) -> Self {
        self.keepna = yes;
        self
    }

    pub async fn fetch(self) -> Result<Vec<Candle>, YfError> {
        let resp = self.fetch_full().await?;
        Ok(resp.candles)
    }

    pub async fn fetch_full(self) -> Result<HistoryResponse, YfError> {
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
                return Err(YfError::Data("no range or period set".into()));
            }

            qp.append_pair("interval", self.interval.as_str());
            if self.include_actions {
                qp.append_pair("events", "div|split");
            }
            qp.append_pair(
                "includePrePost",
                if self.include_prepost {
                    "true"
                } else {
                    "false"
                },
            );
        }

        let resp = self.client.http().get(url.clone()).send().await?;
        if !resp.status().is_success() {
            return Err(YfError::Status {
                status: resp.status().as_u16(),
                url: url.to_string(),
            });
        }
        let body =
            crate::internal::net::get_text(resp, "history_chart", &self.symbol, "json").await?;
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

        let result = chart
            .result
            .ok_or_else(|| YfError::Data("missing result".into()))?;
        let r0 = result
            .first()
            .ok_or_else(|| YfError::Data("empty result".into()))?;

        let ts = r0.timestamp.as_deref().unwrap_or(&[]);
        let q = r0
            .indicators
            .quote
            .first()
            .ok_or_else(|| YfError::Data("missing quote".into()))?;

        let mut actions_out: Vec<Action> = Vec::new();
        let mut split_events: Vec<(i64, f64)> = Vec::new();

        if let Some(ev) = r0.events.as_ref() {
            if let Some(divs) = ev.dividends.as_ref() {
                for (k, d) in divs {
                    let ts = k.parse::<i64>().unwrap_or(d.date.unwrap_or(0));
                    if let Some(amount) = d.amount {
                        actions_out.push(Action::Dividend { ts, amount });
                    }
                }
            }
            if let Some(splits) = ev.splits.as_ref() {
                for (k, s) in splits {
                    let ts = k.parse::<i64>().unwrap_or(s.date.unwrap_or(0));
                    let (num, den) = if let (Some(n), Some(d)) = (s.numerator, s.denominator) {
                        (n as u32, d as u32)
                    } else if let Some(r) = s.split_ratio.as_deref() {
                        let mut it = r.split('/');
                        let n = it.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(1);
                        let d = it.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(1);
                        (n, d)
                    } else {
                        (1, 1)
                    };
                    actions_out.push(Action::Split {
                        ts,
                        numerator: num,
                        denominator: den,
                    });
                    let ratio = if den == 0 {
                        1.0
                    } else {
                        (num as f64) / (den as f64)
                    };
                    split_events.push((ts, ratio));
                }
            }
        }
        actions_out.sort_by_key(|a| match a {
            Action::Dividend { ts, .. } => *ts,
            Action::Split { ts, .. } => *ts,
        });
        split_events.sort_by_key(|(ts, _)| *ts);

        let adj = r0.indicators.adjclose.first();
        let adj_vec = adj.map(|a| a.adjclose.as_slice()).unwrap_or(&[]);

        let mut cum_split_after: Vec<f64> = vec![1.0; ts.len()];
        if !split_events.is_empty() && !ts.is_empty() {
            let mut sp = split_events.len() as isize - 1;
            let mut running: f64 = 1.0;
            for i in (0..ts.len()).rev() {
                while sp >= 0 && split_events[sp as usize].0 > ts[i] {
                    running *= split_events[sp as usize].1;
                    sp -= 1;
                }
                cum_split_after[i] = running;
            }
        }

        let mut out = Vec::new();
        let mut raw_close_vec = Vec::new();

        for (i, &t) in ts.iter().enumerate() {
            let getter_f64 = |v: &Vec<Option<f64>>| v.get(i).and_then(|x| *x);
            let mut open = getter_f64(&q.open);
            let mut high = getter_f64(&q.high);
            let mut low = getter_f64(&q.low);
            let mut close = getter_f64(&q.close);
            let volume0 = q.volume.get(i).and_then(|x| *x);

            // capture raw close for back_adjust
            let raw_close_val = close.unwrap_or(f64::NAN);

            if self.auto_adjust {
                // compute adjust factor
                let factor_from_adj = if let Some(adjclose) = adj_vec.get(i).and_then(|x| *x) {
                    if let Some(c) = close {
                        if c != 0.0 { Some(adjclose / c) } else { None }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let price_factor =
                    factor_from_adj.unwrap_or_else(|| 1.0 / cum_split_after[i].max(1e-12));

                if let Some(v) = open.as_mut() {
                    *v *= price_factor;
                }
                if let Some(v) = high.as_mut() {
                    *v *= price_factor;
                }
                if let Some(v) = low.as_mut() {
                    *v *= price_factor;
                }
                if let Some(v) = close.as_mut() {
                    *v *= price_factor;
                }

                // volume adjusts for splits only
                let volume_adj = volume0.map(|v| {
                    let v_adj = (v as f64) * cum_split_after[i];
                    if v_adj.is_finite() {
                        v_adj.round() as u64
                    } else {
                        v
                    }
                });

                if let (Some(open_v), Some(high_v), Some(low_v), Some(close_v)) =
                    (open, high, low, close)
                {
                    out.push(Candle {
                        ts: t,
                        open: open_v,
                        high: high_v,
                        low: low_v,
                        close: close_v,
                        volume: volume_adj,
                    });
                    raw_close_vec.push(raw_close_val);
                } else if self.keepna {
                    out.push(Candle {
                        ts: t,
                        open: open.unwrap_or(f64::NAN),
                        high: high.unwrap_or(f64::NAN),
                        low: low.unwrap_or(f64::NAN),
                        close: close.unwrap_or(f64::NAN),
                        volume: volume0, // keep as-is when NA row
                    });
                    raw_close_vec.push(raw_close_val);
                }
            } else {
                // no adjustment at all
                if let (Some(open_v), Some(high_v), Some(low_v), Some(close_v)) =
                    (open, high, low, close)
                {
                    out.push(Candle {
                        ts: t,
                        open: open_v,
                        high: high_v,
                        low: low_v,
                        close: close_v,
                        volume: volume0,
                    });
                    raw_close_vec.push(raw_close_val);
                } else if self.keepna {
                    out.push(Candle {
                        ts: t,
                        open: open.unwrap_or(f64::NAN),
                        high: high.unwrap_or(f64::NAN),
                        low: low.unwrap_or(f64::NAN),
                        close: close.unwrap_or(f64::NAN),
                        volume: volume0,
                    });
                    raw_close_vec.push(raw_close_val);
                }
            }
        }

        let meta_out = r0.meta.as_ref().map(|m| HistoryMeta {
            timezone: m.timezone.clone(),
            gmtoffset: m.gmtoffset,
        });

        Ok(HistoryResponse {
            candles: out,
            actions: actions_out,
            adjusted: self.auto_adjust,
            meta: meta_out,
            raw_close: Some(raw_close_vec),
        })
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
    meta: Option<MetaNode>,
    #[serde(default)]
    timestamp: Option<Vec<i64>>,
    indicators: Indicators,
    #[serde(default)]
    events: Option<Events>,
}

#[derive(Deserialize)]
struct MetaNode {
    #[serde(default)]
    timezone: Option<String>,
    #[serde(default)]
    gmtoffset: Option<i64>,
}

#[derive(Deserialize)]
struct Indicators {
    #[serde(default)]
    quote: Vec<QuoteBlock>,
    #[serde(default)]
    adjclose: Vec<AdjCloseBlock>,
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

#[derive(Deserialize)]
struct AdjCloseBlock {
    #[serde(default)]
    adjclose: Vec<Option<f64>>,
}

#[derive(Deserialize, Default)]
struct Events {
    #[serde(default)]
    dividends: Option<std::collections::BTreeMap<String, DividendEvent>>,
    #[serde(default)]
    splits: Option<std::collections::BTreeMap<String, SplitEvent>>,
}

#[derive(Deserialize)]
struct DividendEvent {
    amount: Option<f64>,
    date: Option<i64>,
}

#[derive(Deserialize)]
struct SplitEvent {
    numerator: Option<u64>,
    denominator: Option<u64>,
    #[serde(rename = "splitRatio")]
    split_ratio: Option<String>,
    date: Option<i64>,
}
