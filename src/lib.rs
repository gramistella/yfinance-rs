//! yfinance-rs: ergonomic Yahoo Finance client.
//!
//! Stage 1: daily OHLCV history via the chart v8 endpoint.

pub mod client;
pub mod error;
pub mod history;
pub(crate) mod internal {
    pub(crate) mod net;
    #[cfg(feature = "test-mode")]
    pub(crate) mod fixtures;
}
pub mod profile;

pub use client::YfClient;
pub use error::YfError;
pub use history::{HistoryBuilder, Range, Candle};
pub use profile::{Address, Company, Fund, Profile};

#[cfg(feature = "test-mode")]
pub use client::ApiPreference;