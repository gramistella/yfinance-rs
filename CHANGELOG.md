# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---
## [0.1.3] - 2025-08-31

### Changed
- **Ergonomics Improvement**: Updated `Ticker::new` method to accept a reference to `YfClient` instead of taking ownership.
- **Builder Pattern Enhancement**: Modified `SearchBuilder` to accept a reference to `YfClient`.

### Added
- **Enhanced Core Module**: Added `CacheMode` and `RetryConfig` re-exports to the core module for easier access.

## [0.1.2] - 2025-08-30

### Added
- **New Examples**: Added new examples (`10_convenience_methods.rs`, `11_builder_configuration.rs`, `12_advanced_client.rs`) to showcase convenience methods, advanced builder usage, cache management, and error handling.
- **Development Tooling**: Added `justfile` recipes for linting (`lint`, `lint-fix`, `lint-strict`) to streamline the development and contribution workflow.

### Changed
- **Improved Ergonomics**: Re-exported `YfClientBuilder` at the crate root, allowing for a simpler import (`use yfinance_rs::YfClientBuilder`).
- **Internal Refactoring**: Centralized raw data types (e.g., `RawNum`) into a single `src/core/wire.rs` module to eliminate code duplication and improve maintainability.
- **API Update**: Adapted the analyst recommendations API to use the `financialData` field, replacing the incorrect `recommendationMean` field to align with the data source.
- **Debug Output**: Gated all debug file dumps behind the `debug-dumps` feature flag to prevent unintended file system writes.

### Fixed
- **StreamBuilder Ownership**: Corrected an ownership issue in `StreamBuilder` that caused an unnecessary mutable borrow

## [0.1.1] - 2025-08-28

### Added
- **Earnings Trend Data**: You can now fetch analyst earnings and revenue estimates using `ticker.earnings_trend()`.
- **Shares Outstanding**: Added `ticker.shares()` and `ticker.quarterly_shares()` to get historical data on shares outstanding.
- **Capital Gains**: Capital gains distributions are now available through `ticker.capital_gains()` and are included in `ticker.actions()`.
- **Documentation**: Added comprehensive doc comments for the newly introduced public structs (`EarningsTrendRow`, `ShareCount`) and the `Action::CapitalGain` enum variant.

---
## [0.1.0] - 2025-08-27

### Added
- **Initial Release**: First version of the `yfinance-rs` library.
- **Core Functionality**: Fetch comprehensive ticker information (`info`), historical price data (`history`), and real-time quotes (`quote`, `fast_info`).
- **Advanced Data**: Access to options chains (`options`, `option_chain`), company news (`news`), and financial statements (`income_stmt`, `balance_sheet`, `cashflow`).
- **Analysis Tools**: Get analyst recommendations (`recommendations`), ESG sustainability scores (`sustainability`), and holder information (`major_holders`, `institutional_holders`).
- **Utilities**: Support for multi-symbol data downloads (`DownloadBuilder`), real-time data streaming (`StreamBuilder`), and ticker search (`SearchBuilder`).