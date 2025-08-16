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
pub mod fundamentals;
pub mod analysis;
pub mod stream;

pub use client::YfClient;
pub use error::YfError;
pub use history::{Action, Candle, HistoryBuilder, HistoryMeta, HistoryResponse, Range, Interval};
pub use profile::{Address, Company, Fund, Profile};
pub use ticker::{Ticker, Quote, FastInfo, OptionChain, OptionContract};
pub use download::{DownloadBuilder, DownloadResult};
pub use fundamentals::{
    BalanceSheetRow, Calendar as FundCalendar, CashflowRow, Earnings, EarningsQuarter,
    EarningsQuarterEps, EarningsYear, IncomeStatementRow, Num,
};
pub use analysis::{RecommendationRow, RecommendationSummary, UpgradeDowngradeRow, PriceTarget};
pub use stream::{StreamBuilder, StreamConfig, StreamHandle, QuoteUpdate};


#[cfg(feature = "test-mode")]
pub use client::ApiPreference;


