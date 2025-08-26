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
//! * **Historical Data**: Fetch daily, weekly, or monthly OHLCV data for any ticker.
//! * **Real-time Streaming**: Get live quote updates using WebSockets (with an HTTP polling fallback).
//! * **Company Profiles**: Retrieve detailed information about companies and funds.
//! * **Options Chains**: Fetch expiration dates and full option chains (calls and puts).
//! * **Financials**: Access income statements, balance sheets, and cash flow statements (annual & quarterly).
//! * **Analyst Ratings**: Get price targets, recommendations, and upgrade/downgrade history.
//! * **Async API**: Built on `tokio` and `reqwest` for non-blocking I/O.
//! * **High-Level `Ticker` Interface**: A convenient, yfinance-like struct for accessing all data for a single symbol.
//! * **Builder Pattern**: Fluent builders for constructing complex queries.
//! * **In-memory Caching**: Optional caching to reduce redundant network requests.
//! * **Configurable Retries**: Automatic retries with exponential backoff for transient network errors.
//!
//! ## Quick Start
//!
//! To get started, add `yfinance-rs` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! yfinance-rs = "0.1.0"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! Then, create a `YfClient` and use a `Ticker` to fetch data.
//!
//! ```no_run
//! use yfinance_rs::{Interval, Ticker, YfClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = YfClient::default();
//!     let ticker = Ticker::new(client, "AAPL".to_string());
//!
//!     // Get the latest quote
//!     let quote = ticker.quote().await?;
//!     println!("Latest price for AAPL: ${:.2}", quote.regular_market_price.unwrap_or(0.0));
//!
//!     // Get historical data for the last 6 months
//!     let history = ticker.history(None, Some(Interval::D1), false).await?;
//!     if let Some(last_bar) = history.last() {
//!         println!("Last closing price: ${:.2} on timestamp {}", last_bar.close, last_bar.ts);
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
/// Stream real-time quote updates via WebSockets or polling.
pub mod stream;
/// A high-level interface for a single ticker, providing access to all data types.
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
pub use holders::{
    HoldersBuilder, InsiderRosterHolder, InsiderTransaction, InstitutionalHolder, MajorHolder,
    NetSharePurchaseActivity,
};
pub use news::{NewsArticle, NewsBuilder, NewsTab};
pub use profile::{Address, Company, Fund, Profile};
pub use quote::{QuotesBuilder, quotes};
pub use search::{SearchBuilder, SearchQuote, SearchResponse};
pub use stream::{QuoteUpdate, StreamBuilder, StreamConfig, StreamHandle, StreamMethod};
pub use ticker::{FastInfo, OptionChain, OptionContract, Ticker};

#[cfg(feature = "test-mode")]
#[doc(hidden)]
pub use core::client::ApiPreference;
