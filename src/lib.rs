//! yfinance-rs: ergonomic Yahoo Finance client.
//!
//! Stage 1: daily OHLCV history via the chart v8 endpoint.

pub mod client;
pub mod error;
pub mod history;
pub(crate) mod net;
pub mod profile;
pub mod types;

pub use client::YfClient;
pub use error::YfError;
pub use history::{HistoryBuilder, Range};
pub use profile::{Address, Company, Fund, Profile};
pub use types::Candle;

#[cfg(feature = "test-mode")]
pub use client::ApiPreference;