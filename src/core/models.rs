// Re-export types from borsa-types
pub use borsa_types::Quote;

// Re-export history types from borsa-types
pub use borsa_types::{Action, Candle, HistoryMeta, HistoryResponse};

// Re-export history parameter types from borsa-types
pub use borsa_types::{Interval, Range};

// Helper functions for converting to string representations
pub(crate) fn range_as_str(range: Range) -> &'static str {
    match range {
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

pub(crate) fn interval_as_str(interval: Interval) -> &'static str {
    match interval {
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
