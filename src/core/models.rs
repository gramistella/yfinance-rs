use serde::Serialize;

/* ----- QUOTES (shared by quote/, ticker/, stream/) ----- */
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Quote {
    pub symbol: String,
    pub regular_market_price: Option<f64>,
    pub regular_market_previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,
}

/* ----- HISTORY (shared by history/ and download/) ----- */
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candle {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Action {
    Dividend {
        ts: i64,
        amount: f64,
    },
    Split {
        ts: i64,
        numerator: u32,
        denominator: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryMeta {
    pub timezone: Option<String>,
    pub gmtoffset: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryResponse {
    pub candles: Vec<Candle>,
    pub actions: Vec<Action>,
    pub adjusted: bool,
    pub meta: Option<HistoryMeta>,
    pub raw_close: Option<Vec<f64>>,
}

/* ----- HISTORY PARAMS (so download/ doesn’t import history/) ----- */
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
    pub(crate) fn as_str(self) -> &'static str {
        /* copy from old history/params.rs */
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
    pub(crate) fn as_str(self) -> &'static str {
        /* copy from old history/params.rs */
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
