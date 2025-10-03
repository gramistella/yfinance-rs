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
//! * **Dividend History**: Fetch complete dividend payment history with amounts and dates.
//! * **Stock Splits**: Get stock split history with split ratios.
//! * **Capital Gains**: Retrieve capital gains distributions (especially for mutual funds).
//! * **All Corporate Actions**: Comprehensive access to dividends, splits, and capital gains in one call.
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
//! * **ESG Scores**: Fetch detailed Environmental, Social, and Governance ratings.
//! * **ESG Involvement**: Specific ESG involvement and controversy data.
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
//! * **Data Repair**: Automatic detection and repair of price outliers.
//! * **Data Rounding**: Control price precision and rounding.
//! * **Missing Data Handling**: Configurable handling of NA/missing values.
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
//! ## Quick Start
//!
//! To get started, add `yfinance-rs` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! yfinance-rs = "0.3.2"
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
//!     println!("Latest price for AAPL: ${:.2}", quote.price.as_ref().map(|p| yfinance_rs::core::conversions::money_to_f64(p)).unwrap_or(0.0));
//!
//!     // Get historical data for the last 6 months
//!     let history = ticker.history(Some(Range::M6), Some(Interval::D1), false).await?;
//!     if let Some(last_bar) = history.last() {
//!         println!("Last closing price: ${:.2} on timestamp {}", yfinance_rs::core::conversions::money_to_f64(&last_bar.close), last_bar.ts);
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
/// Search for tickers by name or keyword.
pub mod search;
/// Stream real-time quote updates via `WebSockets` or polling.
pub mod stream;
/// A high-level interface for a single ticker, providing access to all data types.
pub mod ticker;

// --- re-exports (public API remains the same names as before) ---
// Core types that are provider-specific
pub use core::client::ApiPreference;
pub use core::{CacheMode, RetryConfig, YfClient, YfClientBuilder, YfError};

// Provider-specific builders and utilities
pub use download::{DownloadBuilder, DownloadResult};
pub use esg::EsgBuilder;
pub use fundamentals::FundamentalsBuilder;
pub use history::HistoryBuilder;
pub use holders::HoldersBuilder;
pub use news::{NewsBuilder, NewsTab};
pub use quote::{QuotesBuilder, quotes};
pub use search::{SearchBuilder, search};
pub use stream::{StreamBuilder, StreamConfig, StreamHandle, StreamMethod};
pub use ticker::{FastInfo, Info, Ticker};

// Explicitly re-export selected paft core types commonly used by users of this crate
pub use crate::core::{Action, Candle, HistoryMeta, HistoryResponse, Quote};
pub use crate::core::{Interval, Range};
