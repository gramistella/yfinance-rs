use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

/// A single daily OHLCV bar.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candle {
    /// Seconds since Unix epoch (UTC) as returned by the endpoint.
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
