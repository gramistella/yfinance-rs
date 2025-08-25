//! yfinance-rs: ergonomic Yahoo Finance client.
//!
//! Stage 1: daily OHLCV history via the chart v8 endpoint.

pub mod core;

// --- feature modules ---
pub mod analysis;
pub mod download;
pub mod fundamentals;
pub mod history;
pub mod profile;
pub mod quote;
pub mod search;
pub mod stream;
pub mod ticker;

// --- re-exports (public API remains the same names as before) ---
pub use analysis::{PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};
pub use core::{
    Action, Candle, HistoryMeta, HistoryResponse, Interval, Quote, Range, YfClient, YfError,
};
pub use download::{DownloadBuilder, DownloadResult};
pub use fundamentals::{
    BalanceSheetRow, Calendar as FundCalendar, CashflowRow, Earnings, EarningsQuarter,
    EarningsQuarterEps, EarningsYear, FundamentalsBuilder, IncomeStatementRow, Num,
};
pub use history::HistoryBuilder;
pub use profile::{Address, Company, Fund, Profile};
pub use quote::{QuotesBuilder, quotes};
pub use search::{SearchBuilder, SearchQuote, SearchResponse};
pub use stream::{QuoteUpdate, StreamBuilder, StreamConfig, StreamHandle, StreamMethod};
pub use ticker::{FastInfo, OptionChain, OptionContract, Ticker};

#[cfg(feature = "test-mode")]
pub use core::client::ApiPreference;
