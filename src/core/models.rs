// Re-export types from paft explicitly
pub use paft::market::action::Action;
pub use paft::market::quote::Quote;
pub use paft::market::requests::history::{Interval, Range};
pub use paft::market::responses::history::{Candle, HistoryMeta, HistoryResponse};

// Helper functions for converting to string representations
pub(crate) const fn range_as_str(range: Range) -> &'static str {
    match range {
        Range::I1m => "1m",
        Range::I2m => "2m",
        Range::I5m => "5m",
        Range::I10m => "10m",
        Range::I15m => "15m",
        Range::I30m => "30m",
        Range::I1h => "1h",
        Range::I4h => "4h",
        Range::I6h => "6h",
        Range::I8h => "8h",
        Range::I12h => "12h",
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

pub(crate) const fn interval_as_str(interval: Interval) -> &'static str {
    match interval {
        Interval::I1s => "1s",
        Interval::I2s => "2s",
        Interval::I3s => "3s",
        Interval::I5s => "5s",
        Interval::I6s => "6s",
        Interval::I10s => "10s",
        Interval::I15s => "15s",
        Interval::I30s => "30s",

        Interval::I90s => "90s",
        Interval::I1m => "1m",
        Interval::I2m => "2m",
        Interval::I3m => "3m",
        Interval::I5m => "5m",
        Interval::I6m => "6m",
        Interval::I10m => "10m",
        Interval::I15m => "15m",
        Interval::I30m => "30m",
        Interval::I90m => "90m",
        Interval::I1h => "1h",
        Interval::I2h => "2h",
        Interval::I3h => "3h",
        Interval::I4h => "4h",
        Interval::I6h => "6h",
        Interval::I8h => "8h",
        Interval::I12h => "12h",
        Interval::D1 => "1d",
        Interval::D5 => "5d",
        Interval::W1 => "1wk",
        Interval::M1 => "1mo",
        Interval::M3 => "3mo",
        Interval::M6 => "6mo",
        Interval::Y1 => "1y",
        Interval::Y2 => "2y",
        Interval::Y5 => "5y",
        Interval::Y10 => "10y",
    }
}
