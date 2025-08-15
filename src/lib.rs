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
pub mod ticker;
pub mod download;

pub use client::YfClient;
pub use error::YfError;
pub use history::{Action, Candle, HistoryBuilder, HistoryMeta, HistoryResponse, Range, Interval};
pub use profile::{Address, Company, Fund, Profile};
pub use ticker::{Ticker, Quote, FastInfo, OptionChain, OptionContract};
pub use download::{DownloadBuilder, DownloadResult};

#[cfg(feature = "test-mode")]
pub use client::ApiPreference;


