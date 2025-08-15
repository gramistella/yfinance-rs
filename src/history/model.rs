use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candle {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<u64>,
}

impl Candle {
    pub fn datetime_utc(&self) -> DateTime<Utc> {
        Utc.timestamp_opt(self.ts, 0).single().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Action {
    Dividend { ts: i64, amount: f64 },
    Split { ts: i64, numerator: u32, denominator: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryResponse {
    pub candles: Vec<Candle>,
    pub actions: Vec<Action>,
    /// true when prices are auto-adjusted (for splits & dividends)
    pub adjusted: bool,
}