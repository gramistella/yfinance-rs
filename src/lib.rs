//! # yfinance-rs
//!
//! An ergonomic, async-first Rust client for the unofficial Yahoo Finance API.
//!
//! This crate provides a simple and efficient way to fetch financial data from Yahoo Finance.
//! It is designed to feel familiar to users of the popular Python `yfinance` library, but
//! leverages Rust's powerful type system and async capabilities for performance and safety.
//!
//! ## Features
//!
//! ### Core Data
//! * **Historical Data**: Fetch daily, weekly, or monthly OHLCV data with automatic split/dividend adjustments.
//! * **Real-time Quotes**: Get live quote updates with detailed market information.
//! * **Fast Quotes**: Optimized quote fetching with essential data only (`fast_info`).
//! * **Multi-Symbol Downloads**: Concurrently download historical data for many symbols at once.
//! * **Batch Quotes**: Fetch quotes for multiple symbols efficiently.
//!
//! ### Corporate Actions & Dividends
//! * **Typed Corporate Actions**: Fetch dividends, splits, and capital gains through one currency-aware action stream.
//!
//! ### Financial Statements & Fundamentals
//! * **Income Statements**: Access annual and quarterly income statements.
//! * **Balance Sheets**: Get annual and quarterly balance sheet data.
//! * **Cash Flow Statements**: Fetch annual and quarterly cash flow data.
//! * **Earnings Data**: Historical earnings, revenue estimates, and EPS data.
//! * **Shares Outstanding**: Historical data on shares outstanding (annual and quarterly).
//! * **Corporate Calendar**: Earnings dates, ex-dividend dates, and dividend payment dates.
//!
//! ### Options & Derivatives
//! * **Options Chains**: Fetch expiration dates and full option chains (calls and puts).
//! * **Option Contracts**: Detailed option contract information.
//!
//! ### Analysis & Research
//! * **Analyst Ratings**: Get price targets, recommendations, and upgrade/downgrade history.
//! * **Earnings Trends**: Detailed earnings and revenue estimates from analysts.
//! * **Recommendations Summary**: Summary of current analyst recommendations.
//! * **Upgrades/Downgrades**: History of analyst rating changes.
//!
//! ### Ownership & Holders
//! * **Major Holders**: Get major, institutional, and mutual fund holder data.
//! * **Institutional Holders**: Top institutional shareholders and their holdings.
//! * **Mutual Fund Holders**: Mutual fund ownership breakdown.
//! * **Insider Transactions**: Recent insider buying and selling activity.
//! * **Insider Roster**: Company insiders and their current holdings.
//! * **Net Share Activity**: Summary of insider purchase/sale activity.
//!
//! ### ESG & Sustainability
//! * **ESG Scores**: Fetch Environmental, Social, and Governance ratings when Yahoo returns them.
//! * **ESG Involvement**: Specific ESG involvement and controversy data when Yahoo returns it.
//!
//! ### News & Information
//! * **Company News**: Retrieve the latest articles and press releases for a ticker.
//! * **Company Profiles**: Detailed information about companies, ETFs, and funds.
//! * **Search**: Find tickers by name or keyword.
//!
//! ### Real-time Streaming
//! * **WebSocket Streaming**: Get live quote updates using `WebSockets` (preferred method).
//! * **HTTP Polling**: Fallback polling method for real-time data.
//! * **Configurable Streaming**: Customize update frequency and change-only filtering.
//!
//! ### Advanced Features
//! * **Data Rounding**: Control price precision and rounding.
//! * **Malformed Data Handling**: Drops invalid OHLC rows while preserving valid sibling data.
//! * **Back Adjustment**: Alternative price adjustment methods.
//! * **Historical Metadata**: Timezone and other metadata for historical data.
//! * **ISIN Lookup**: Get International Securities Identification Numbers.
//!
//! ### Developer Experience
//! * **Async API**: Built on `tokio` and `reqwest` for non-blocking I/O.
//! * **High-Level `Ticker` Interface**: A convenient, yfinance-like struct for accessing all data for a single symbol.
//! * **Builder Pattern**: Fluent builders for constructing complex queries.
//! * **Configurable Retries**: Automatic retries with exponential backoff for transient network errors.
//! * **Caching**: Configurable caching behavior for API responses.
//! * **Custom Timeouts**: Configurable request timeouts and connection settings.
//!
//! ## Cargo Features
//!
//! User-facing optional features:
//! * `stream`: enables WebSocket streaming support.
//! * `dataframe`: re-exports `paft` `DataFrame` conversion traits.
//! * `tracing`: compiles structured tracing instrumentation.
//!
//! Internal/unstable features:
//! * `test-mode`: enables repository test hooks, fixture recording, and
//!   doc-hidden plumbing APIs for yfinance-rs' own integration tests.
//! * `debug-dumps`: enables maintainer diagnostics for dumping selected raw
//!   Yahoo responses.
//! * `tracing-subscriber`: test/example convenience for initializing a basic
//!   subscriber; applications should configure their own subscriber instead.
//!
//! Internal features are published only because Cargo has no private feature
//! namespace. They are not part of the supported user-facing API.
//!
//! ## Quick Start
//!
//! To get started, add `yfinance-rs` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! yfinance-rs = "0.10.0"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! Then, create a `YfClient` and use a `Ticker` to fetch data.
//!
//! ```no_run
//! use yfinance_rs::{Interval, Range, Ticker, YfClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = YfClient::default();
//!     let ticker = Ticker::new(&client, "AAPL");
//!
//!     // Get the latest quote
//!     let quote = ticker.quote().await?;
//!     if let Some(price) = quote.price.as_ref() {
//!         println!("Latest price for AAPL: {price}");
//!     }
//!
//!     // Get historical data for the last 6 months
//!     let history = ticker.history(Some(Range::M6), Some(Interval::D1), false).await?;
//!     if let Some(last_bar) = history.last() {
//!         println!("Last closing price: {} on timestamp {}", last_bar.ohlc.close, last_bar.ts);
//!     }
//!
//!     // Get analyst recommendations
//!     let recs = ticker.recommendations().await?;
//!     if let Some(latest_rec) = recs.first() {
//!         println!("Latest recommendation period: {}", latest_rec.period);
//!     }
//!
//!     Ok(())
//! }
//! ```
#![warn(missing_docs)]

/// Core components, including the `YfClient` and `YfError`.
pub mod core;

// --- feature modules ---
/// Fetch analyst ratings, price targets, and upgrade/downgrade history.
pub mod analysis;
/// Download historical data for multiple symbols concurrently.
pub mod download;
/// Fetch ESG (Environmental, Social, Governance) scores and involvement data.
pub mod esg;
/// Fetch financial statements (income, balance sheet, cash flow) and earnings data.
pub mod fundamentals;
/// Fetch historical OHLCV data for a single symbol.
pub mod history;
/// Fetch holder information, including major, institutional, and insider holders.
pub mod holders;
/// Fetch news articles for a ticker.
pub mod news;
/// Retrieve company or fund profile information.
pub mod profile;
/// Fetch quotes for multiple symbols.
pub mod quote;
/// Run Yahoo Finance predefined and custom screeners.
pub mod screener;
/// Search for tickers by name or keyword.
pub mod search;
/// Stream real-time quote updates via `WebSockets` or polling.
#[cfg(feature = "stream")]
pub mod stream;
/// A high-level interface for a single ticker, providing access to all data types.
pub mod ticker;

// Core types that are provider-specific
pub use core::{
    Backoff, CacheEndpoint, CacheMode, DataQuality, HistoryRequest, HistoryService,
    ProjectionIssue, RetryConfig, YfClient, YfClientBuilder, YfCurrencyInference,
    YfCurrencyPurpose, YfDiagnostics, YfError, YfResponse, YfWarning,
};

/// `DataFrame` conversion traits re-exported from `paft`.
#[cfg(feature = "dataframe")]
pub mod dataframe {
    pub use paft::dataframe::*;
}

#[cfg(feature = "dataframe")]
pub use dataframe::{Columnar, Decimal128Encode, ToDataFrame, ToDataFrameVec};

// Provider-specific builders and utilities
pub use analysis::AnalysisBuilder;
pub use download::{DownloadAdjustment, DownloadBuilder, DownloadConcurrency};
pub use esg::EsgBuilder;
pub use fundamentals::FundamentalsBuilder;
pub use history::HistoryBuilder;
pub use holders::HoldersBuilder;
pub use news::{NewsBuilder, NewsTab};
pub use quote::{QuotesBuilder, quotes, quotes_with_diagnostics};
pub use screener::{
    EquityQuery, EquitySector, EtfCategory, EtfQuery, FundCategory, FundQuery, PercentPoints,
    PredefinedScreener, Rating, Region, ResultOffset, ScreenerBuilder, ScreenerCount,
    ScreenerNumber, ScreenerQuery, ScreenerResponse, ScreenerResult, SortDirection,
    YahooExchangeCode, YahooQuoteType, equity_fields, etf_fields, fund_fields, screen,
    screen_with_diagnostics,
};
pub use search::{SearchBuilder, search};
#[cfg(feature = "stream")]
pub use stream::{StreamBuilder, StreamConfig, StreamHandle, StreamMethod};
pub use ticker::{FastInfo, Info, MovingAverages, Ticker};

/// Initialize a default tracing subscriber for tests/examples when the
/// `tracing-subscriber` feature is enabled. No-op otherwise.
#[cfg(feature = "tracing-subscriber")]
#[doc(hidden)]
pub fn init_tracing_for_tests() {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| EnvFilter::try_new(s).ok())
        .unwrap_or_else(|| EnvFilter::new("info"));
    let _ = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(true)
        .with_ansi(true)
        .try_init();
}

// Explicitly re-export selected paft types exposed by this crate's public API.
pub use crate::core::{
    Action, AdjustmentAnchor, AdjustmentMethod, Candle, CorporateActionAdjustmentCause,
    CorporateActionAdjustmentCauses, HistoryMeta, HistoryResponse, OhlcPriceBasis, PriceBasis,
    Quote,
};
pub use crate::core::{Interval, Range};
pub use paft::aggregates::Snapshot;
pub use paft::domain::{
    AssetKind, Exchange, Instrument, Isin, MarketState, PeriodYear, ReportingPeriod, Symbol,
};
pub use paft::fundamentals::analysis::{
    Earnings, EarningsQuarter, EarningsQuarterEps, EarningsTrendRow, EarningsYear, PriceTarget,
    RecommendationAction, RecommendationGrade, RecommendationRow, RecommendationSummary,
    UpgradeDowngradeRow,
};
pub use paft::fundamentals::esg::{EsgInvolvement, EsgScores, EsgSummary};
pub use paft::fundamentals::holders::{
    InsiderPosition, InsiderRosterHolder, InsiderTransaction, InstitutionalHolder, MajorHolder,
    NetSharePurchaseActivity, TransactionType,
};
pub use paft::fundamentals::profile::{
    Address, CompanyProfile as Company, FundKind, FundProfile as Fund, Profile, ShareCount,
};
pub use paft::fundamentals::statements::{
    BalanceSheetRow, Calendar, CashflowRow, IncomeStatementRow,
};
pub use paft::fundamentals::statistics::KeyStatistics;
pub use paft::market::news::NewsArticle;
pub use paft::market::options::{
    OptionChain, OptionContract, OptionContractKey, OptionGreeks, OptionSide,
};
pub use paft::market::orderbook::BookLevel;
pub use paft::market::quote::QuoteUpdate;
pub use paft::market::responses::download::{DownloadEntry, DownloadResponse};
pub use paft::market::responses::history::Ohlc;
pub use paft::market::responses::search::{SearchResponse, SearchResult};
pub use paft::money::{
    Currency, IsoCurrency, MonetaryAmount, Money, Price, PriceAmount, QuantityAmount,
};
pub use paft::{Decimal, NonNegativeDecimal, Ratio};
