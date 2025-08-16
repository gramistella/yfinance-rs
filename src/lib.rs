//! yfinance-rs: ergonomic Yahoo Finance client.
//!
//! Stage 1: daily OHLCV history via the chart v8 endpoint.

pub mod client;
pub mod error;
pub mod history;
pub(crate) mod internal {
    #[cfg(feature = "test-mode")]
    pub(crate) mod fixtures;
    pub(crate) mod net;
}
pub mod analysis;
pub mod download;
pub mod fundamentals;
pub mod profile;
pub mod quote;
pub mod search;
pub mod stream;
pub mod ticker;

pub use analysis::{PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};
pub use client::YfClient;
pub use download::{DownloadBuilder, DownloadResult};
pub use error::YfError;
pub use fundamentals::{
    BalanceSheetRow, Calendar as FundCalendar, CashflowRow, Earnings, EarningsQuarter,
    EarningsQuarterEps, EarningsYear, IncomeStatementRow, Num,
};
pub use history::{Action, Candle, HistoryBuilder, HistoryMeta, HistoryResponse, Interval, Range};
pub use profile::{Address, Company, Fund, Profile};
pub use quote::{QuotesBuilder, quotes};
pub use search::{SearchBuilder, SearchQuote, SearchResponse};
pub use stream::{QuoteUpdate, StreamBuilder, StreamConfig, StreamHandle};
pub use ticker::{FastInfo, OptionChain, OptionContract, Quote, Ticker};

#[cfg(feature = "test-mode")]
pub use client::ApiPreference;
