# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-09-16

### Added

- New optional `dataframe` feature: all `paft` data models now support `.to_dataframe()` when the feature is enabled, returning Polars `DataFrame`s. Added example `14_polars_dataframes.rs` and README section.

### Changed

- **Breaking change:** All public data models (such as `Quote`, `HistoryBar`, `EarningsTrendRow`, etc.) now use types from the [`paft`](https://crates.io/crates/paft) crate instead of custom-defined structs. This unifies data structures with other financial Rust libraries and improves interoperability, but may require code changes for downstream users.

## [0.1.3] - 2025-08-31

### Added
- Re-exported `CacheMode` and `RetryConfig` from the `core` module.

### Changed
- `Ticker::new` now takes `&YfClient` instead of taking ownership.
- `SearchBuilder` now takes `&YfClient` instead of taking ownership.

## [0.1.2] - 2025-08-30

### Added
- New examples: `10_convenience_methods.rs`, `11_builder_configuration.rs`, `12_advanced_client.rs`.
- Development tooling: `just` recipes `lint`, `lint-fix`, and `lint-strict`.
- Re-exported `YfClientBuilder` at the crate root (`use yfinance_rs::YfClientBuilder`).

### Changed
- Centralized raw wire types (e.g., `RawNum`) into `src/core/wire.rs`.
- Gated debug file dumps behind the `debug-dumps` feature flag.

### Fixed
- Analyst recommendations now read from `financialData` instead of the incorrect `recommendationMean` field.
- Fixed unnecessary mutable borrow in `StreamBuilder` `run_websocket_stream`

## [0.1.1] - 2025-08-28

### Added
- `ticker.earnings_trend()` for analyst earnings and revenue estimates.
- `ticker.shares()` and `ticker.quarterly_shares()` for historical shares outstanding.
- `ticker.capital_gains()` and inclusion of capital gains in `ticker.actions()`.
- Documentation: added doc comments for `EarningsTrendRow`, `ShareCount`, and `Action::CapitalGain`.

## [0.1.0] - 2025-08-27

### Added
- Initial release of `yfinance-rs`.
- Core functionality: `info`, `history`, `quote`, `fast_info`.
- Advanced data: `options`, `option_chain`, `news`, `income_stmt`, `balance_sheet`, `cashflow`.
- Analysis tools: `recommendations`, `sustainability`, `major_holders`, `institutional_holders`.
- Utilities: `DownloadBuilder`, `StreamBuilder`, `SearchBuilder`.
