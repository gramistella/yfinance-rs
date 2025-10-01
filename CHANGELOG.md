# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2025-10-01

### Changed

- Internal migration to `paft` 0.3.0 without changing the public API surface.
  - Switched internal imports to `paft::domain` (domain types) and `paft::money` (money/currency).
  - Updated internal `Money` construction to the new `Result`-returning API and replaced scalar ops with `try_mul` where appropriate.
- Examples and docs now import DataFrame traits from `paft::prelude::{ToDataFrame, ToDataFrameVec}`.
- Conversion helpers in `core::conversions` now document potential panics if a non-ISO currency lacks registered metadata (behavior aligned with `paft-money`).
- Profile ISIN fields now validate ISIN format using `paft::domain::Isin` - invalid ISINs are filtered out and stored as `None`.

## [0.3.0] - 2025-09-20

### Changed

- Migrated to `paft` 0.2.0 with explicit module paths; removed all `paft::prelude` imports across the codebase, tests, and examples.
- Updated enum/string conversions to use `FromStr/TryFrom` parsing from `paft` 0.2.0 (e.g., `MarketState`, `Exchange`, `Period`, insider/transaction/recommendation types).
- Adjusted `Money` operations to use `try_*` methods and made conversions more robust against non-finite values.
- Consolidated public re-exports under `core::models` (e.g., `Interval`, `Range`, `Quote`, `Action`, `Candle`, `HistoryMeta`, `HistoryResponse`) to provide stable, explicit paths.
- Simplified the Polars example behind the `dataframe` feature to avoid prelude usage and to compile cleanly with the new APIs.

### Fixed

- Updated examples and tests to import `Interval`/`Range` from `yfinance_rs::core` explicitly and to avoid wildcard matches in pattern tests.

### Notes

- This release removes reliance on `paft` preludes and may require users to update imports to explicit module paths if depending on re-exported paft items directly.

## [0.2.1] - 2025-09-18

### Added

- Profile-based reporting currency inference with per-symbol caching. The client now inspects the profile country on first use to determine an appropriate currency and reuses that decision across fundamentals and analysis calls.
- ESG involvement exposure: `Ticker::sustainability()` now returns involvement flags (e.g., tobacco, thermal_coal) alongside component scores via `EsgSummary`.

### Changed

- **Breaking change:** `Ticker` convenience methods for fundamentals and analysis (and their corresponding builders) now accept an extra `Option<Currency>` argument. Pass `None` to use the inferred reporting currency, or `Some(currency)` to override the heuristic explicitly.
- **Breaking change:** `Ticker::sustainability()` and `esg::EsgBuilder::fetch()` now return `EsgSummary` instead of `EsgScores`. Access component values via `summary.scores` and involvement via `summary.involvement`.

## [0.2.0] - 2025-09-16

### Added

- New optional `dataframe` feature: all `paft` data models now support `.to_dataframe()` when the feature is enabled, returning Polars `DataFrame`s. Added example `14_polars_dataframes.rs` and README section.
- Custom HTTP client support via `YfClient::builder().custom_client(...)` for full control over `reqwest` configuration.
- Proxy configuration helpers on the client builder: `.proxy()`, `.https_proxy()`, `.try_proxy()`, `.try_https_proxy()`. Added example `13_custom_client_and_proxy.rs`.
- Explicit `User-Agent` is set on all HTTP/WebSocket requests by default, with `.user_agent(...)` to customize it.
- Improved numeric precision in historical adjustments and conversions using `rust_decimal`.

### Changed

- **Breaking change:** All public data models (such as `Quote`, `HistoryBar`, `EarningsTrendRow`, etc.) now use types from the [`paft`](https://crates.io/crates/paft) crate instead of custom-defined structs. This unifies data structures with other financial Rust libraries and improves interoperability, but may require code changes for downstream users.
- Monetary value handling now uses `paft::Money` with currency awareness across APIs and helpers.
- Consolidated and simplified fundamentals timeseries fetching via a generic helper for consistency.
- Error handling refined: `YfError` variants and messages standardized for 404/429/5xx and unexpected statuses.
- Dependencies updated and internal structure adjusted to support the new features.

### Fixed

- Minor clippy findings and documentation typos.

### Known Issues

- Currency inference relies on company profile metadata. If Yahoo omits or mislabels the headquarters country, the inferred currency can still be incorrect—use the new override parameter to force a specific currency in that case.

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
