use serde::Serialize;

/* ----- QUOTES (shared by quote/, ticker/, stream/) ----- */
/// A snapshot of a security's quote, containing key market data.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Quote {
    /// The ticker symbol of the security.
    pub symbol: String,
    /// The short name of the security.
    pub shortname: Option<String>,
    /// The last traded price in the regular market session.
    pub regular_market_price: Option<f64>,
    /// The closing price of the previous regular market session.
    pub regular_market_previous_close: Option<f64>,
    /// The currency in which the security is traded.
    pub currency: Option<String>,
    /// The full name of the exchange where the security is traded.
    pub exchange: Option<String>,
    /// The current state of the market for this security (e.g., "REGULAR", "PRE", "POST").
    pub market_state: Option<String>,
}

/* ----- HISTORY (shared by history/ and download/) ----- */
/// Represents a single OHLCV (Open, High, Low, Close, Volume) data point for a specific time.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candle {
    /// The Unix timestamp (in seconds) for the beginning of the candle's time period.
    pub ts: i64,
    /// The opening price for the period.
    pub open: f64,
    /// The highest price reached during the period.
    pub high: f64,
    /// The lowest price reached during the period.
    pub low: f64,
    /// The closing price for the period.
    pub close: f64,
    /// The trading volume for the period.
    pub volume: Option<u64>,
}

/// Represents a corporate action, either a dividend or a stock split.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Action {
    /// A cash dividend payment.
    Dividend {
        /// The Unix timestamp (in seconds) of the payment date.
        ts: i64,
        /// The dividend amount per share.
        amount: f64,
    },
    /// A stock split.
    Split {
        /// The Unix timestamp (in seconds) of the split date.
        ts: i64,
        /// The numerator of the split ratio (e.g., 2 for a 2-for-1 split).
        numerator: u32,
        /// The denominator of the split ratio (e.g., 1 for a 2-for-1 split).
        denominator: u32,
    },

    /// Represents a capital gain distribution for a fund.
    CapitalGain {
        /// The Unix timestamp (in seconds) of the distribution date.
        ts: i64,
        /// The distributed gain per share.
        gain: f64,
    },
}

/// Metadata associated with a historical data response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryMeta {
    /// The IANA time zone name for the security's exchange (e.g., "America/New_York").
    pub timezone: Option<String>,
    /// The GMT offset in seconds for the exchange.
    pub gmtoffset: Option<i64>,
}

/// A complete response for a historical data request.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryResponse {
    /// A vector of OHLCV candles.
    pub candles: Vec<Candle>,
    /// A vector of corporate actions (dividends and splits) that occurred during the period.
    pub actions: Vec<Action>,
    /// Indicates whether the `candles` prices have been adjusted for splits and dividends.
    pub adjusted: bool,
    /// Metadata about the historical data.
    pub meta: Option<HistoryMeta>,
    /// A vector of the *raw* (unadjusted) closing prices, corresponding to each candle.
    /// This is populated when `auto_adjust` is true, to support back-adjustment.
    pub raw_close: Option<Vec<f64>>,
}

/* ----- HISTORY PARAMS (so download/ doesn’t import history/) ----- */
/// A relative time range for a historical data request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Range {
    /// 1 day.
    D1,
    /// 5 days.
    D5,
    /// 1 month.
    M1,
    /// 3 months.
    M3,
    /// 6 months.
    M6,
    /// 1 year.
    Y1,
    /// 2 years.
    Y2,
    /// 5 years.
    Y5,
    /// 10 years.
    Y10,
    /// Year to date.
    Ytd,
    /// The maximum available range.
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

/// The time interval for historical data bars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interval {
    /// 1 minute.
    I1m,
    /// 2 minutes.
    I2m,
    /// 5 minutes.
    I5m,
    /// 15 minutes.
    I15m,
    /// 30 minutes.
    I30m,
    /// 60 minutes.
    I60m,
    /// 90 minutes.
    I90m,
    /// 1 hour.
    I1h,
    /// 1 day.
    D1,
    /// 5 days.
    D5,
    /// 1 week.
    W1,
    /// 1 month.
    M1,
    /// 3 months.
    M3,
}

impl Interval {
    pub(crate) fn as_str(self) -> &'static str {
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
