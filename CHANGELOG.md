# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2025-10-28

### Added

- Per-update volume deltas in real-time streaming: `QuoteUpdate.volume` now reflects the delta
  since the previous update for a symbol. First tick per symbol and after a detected reset/rollover
  yields `None`. Applies to both WebSocket and HTTP polling streams.
- Expose intraday cumulative volume on snapshots: populate `Quote.day_volume` from v7 quotes and
  surface it on convenience types (`Ticker::quote()` and `Ticker::info()` as `Info.volume`).
- SearchBuilder accessors: `lang_ref()` and `region_ref()` to inspect configured parameters.
- Populate convenience `Info` with analytics and ESG when available: `price_target`,
  `recommendation_summary`, `esg_scores`.

### Breaking Change

- Upgrade to `paft` v0.7.0 adds a new field to `paft::market::quote::QuoteUpdate`:
  `volume: Option<u64>`. If you construct or exhaustively destructure `QuoteUpdate`, update your
  code to include the new field or use `..`. Stream APIs and typical consumers that only read
  updates are unaffected.

### Changed

- Stream volume semantics: WebSocket and polling streams compute per-update volume deltas. The
  low-level decoder helper remains stateless and always returns `volume = None`.
- Polling stream `diff_only` now emits when either price or volume changes.

### Documentation

- README: added a "Volume semantics" section for streaming; clarified delta behavior and how to
  obtain cumulative volume.
- Examples: updated streaming and convenience examples to display volume; SearchBuilder example now
  demonstrates `lang_ref()`/`region_ref()`.

### Dependencies

- Bump `paft` to `v0.7.0`.

## [0.6.1] - 2025-10-27

### Fixed

- Fixed critical timestamp interpretation bug in WebSocket stream processing: use `DateTime::from_timestamp_millis()` instead of `i64_to_datetime()` to correctly interpret millisecond timestamps, preventing incorrect date values in quote updates

#### Notes

- **WebSocket Stream Timestamps:** Users may occasionally observe `QuoteUpdate` messages arriving via the WebSocket stream with timestamps that are older than previously received messages ("time traveling ticks"), sometimes by significant amounts (minutes or hours). This behavior appears to originate from the **Yahoo Finance data feed itself** and is not a bug introduced by `yfinance-rs`. To provide the most direct representation of the source data, `yfinance-rs` **does not automatically filter** these out-of-order messages. Applications requiring strictly chronological quote updates should implement their own filtering logic based on the timestamp (`ts`) field of the received `QuoteUpdate`.

## [0.6.0] - 2025-10-21

### Breaking Change

- `DownloadBuilder::run()` now returns `paft::market::responses::download::DownloadResponse` with an `entries: Vec<DownloadEntry>` instead of the previous `DownloadResult` maps. Access candles via `entry.history.candles` and the symbol via `entry.instrument.symbol_str()`.

### Changed

- Re-export `DownloadEntry` and `DownloadResponse` at the crate root for convenient imports.
- Examples and tests updated to iterate over `entries` rather than `series`.

### Performance

- Introduced an instrument cache in `YfClient` and populate it opportunistically from v7 quote responses to reduce symbol resolution overhead during multi-symbol downloads.

### Documentation

- Updated README examples to reflect the new `DownloadResponse.entries` usage.

### Dependencies

- Bump `paft` to `v0.6.0`.

## [0.5.2] - 2025-10-20

### Added

- Optional `tracing` feature: emits spans and key events across network I/O and major logical boundaries. Instrumented `send_with_retry`, profile fallback, quote summary fetch (including invalid crumb retry), history `fetch_full`, and `Ticker` public APIs (`info`, `quote`, `history`, etc.). Disabled by default; zero overhead when not enabled.
- Optional `tracing-subscriber` feature (dev/testing): convenience initializer `init_tracing_for_tests()` to set up a basic subscriber in examples/tests. The library itself does not configure a subscriber.

### Dependencies

- Bump `paft` to `v0.5.2`.

### Docs

- Readme now includes a "Tracing" section.

## [0.5.1] - 2025-10-17

### Changed

- Updated to paft v0.5.1

## [0.5.0] - 2025-10-16

### Breaking

- Adopted `paft` 0.5.0 identity and money types across search, streaming, and ticker info. `Quote.symbol`, `SearchResult.symbol`, `OptionContract.contract_symbol`, and `QuoteUpdate.symbol` now use `paft::domain::Symbol`; values are uppercased and validated during construction, and invalid search results are dropped.
- `Ticker::Info` now re-exports `paft::aggregates::Info`. The previous struct with raw strings and floats has been removed, and fields such as `sector`, `industry`, analyst targets, recommendation metrics, and ESG scores are no longer populated on this convenience type. Monetary and exchange data now use `Money`, `Currency`, `Exchange`, and `MarketState`.
- Real-time streaming emits `paft::market::quote::QuoteUpdate`. `last_price` is renamed to `price` and now carries `Money` (with embedded currency metadata), the standalone `currency` string is gone, and `ts` is now a `DateTime<Utc>`. Update stream consumers accordingly.
- Search now returns `paft::market::responses::search::SearchResponse` with a `results` list. Each item exposes `Symbol`, `AssetKind`, and `Exchange` enums. Replace usages of `resp.quotes` and `quote.longname/shortname` with `resp.results` and `result.name`.

### Changed

- Bumped `paft` to 0.5.0 via the workspace checkout and aligned with the new symbol validation.
- Updated dependencies and fixtures: `reqwest 0.12.24`, `tokio 1.48`.

### Documentation

- Added troubleshooting guidance for consent-related errors in `README.md` (thanks to [@hrishim](https://github.com/hrishim) for the contribution!)
- Expanded `CONTRIBUTING.md` with `just` helpers and clarified repository setup.

### Internal

- Added `.github/FUNDING.yml` to advertise GitHub Sponsors support.
- Removed stray `.DS_Store` files and regenerated fixtures for the new models.

### Migration notes

- Symbols are now uppercase-validated `paft::domain::Symbol`. Use `.as_str()` for string comparisons or construct values with `Symbol::new("AAPL")` (handle the `Result` when user input is dynamic).
- Stream updates now expose `update.price` (`Money`) and `update.ts: DateTime<Utc>`. Replace direct `last_price`/`ts` usage with the new typed fields and derive primitive values as needed.
- Search responses provide `resp.results` instead of `resp.quotes`. Access display data via `result.name`, `result.kind`, and `result.exchange`.
- The convenience info snapshot no longer embeds fundamentals, analyst, or ESG data. Fetch those via `profile::load_profile`, `analysis::AnalysisBuilder`, and `esg::EsgBuilder` if you still need them.

---

## [0.4.0] - 2025-10-12

### Added

- Enabled `paft` facade `aggregates` feature.
  - `Ticker::fast_info()` now returns `paft_aggregates::FastInfo` (typed enums and `Money`), offering a richer, consistent snapshot model.
- Options models expanded (re-exported from `paft-market`):
  - `OptionContract` gains `expiration_date` (NaiveDate), `expiration_at` (Option<DateTime\<Utc>>), `last_trade_at` (Option<DateTime\<Utc>>), and `greeks` (Option\<OptionGreeks>).
- DataFrame support for options types is available when enabling this crate’s `dataframe` feature (forwards to `paft/dataframe`).

### Changed

- History response alignment with `paft` 0.4.0:
  - `Candle` now carries `close_unadj: Option<Money>` (original unadjusted close, when available).
  - `HistoryResponse` no longer includes a top-level `unadjusted_close` vector.
- Examples and tests updated to use Money-typed values and typed enums (Exchange, MarketState, Currency).

### Breaking

- Fast Info return type changed:
  - Old: struct with `last_price: f64`, `previous_close: Option<f64>`, string-y `currency`/`exchange`/`market_state`.
  - New: `paft_aggregates::FastInfo` with `last: Option<Money>`, `previous_close: Option<Money>`, `currency: Option<paft_money::Currency>`, `exchange: Option<paft_domain::Exchange>`, `market_state: Option<paft_domain::MarketState>`, plus `name: Option<String>`.
- Options contract fields changed:
  - Old: `OptionContract { ..., expiration: DateTime<Utc>, ... }`
  - New: `OptionContract { ..., expiration_date: NaiveDate, expiration_at: Option<DateTime<Utc>>, last_trade_at: Option<DateTime<Utc>>, greeks: Option<OptionGreeks>, ... }`
- History unadjusted close location changed:
  - Old: `HistoryResponse { ..., unadjusted_close: Option<Vec<Money>> }`
  - New: `Candle { ..., close_unadj: Option<Money> }` (per-candle).

### Migration notes

- Fast Info
  - Price as f64: replace `fi.last_price` with `fi.last.as_ref().map(money_to_f64).or_else(|| fi.previous_close.as_ref().map(money_to_f64))`.
  - Currency string: replace `fi.currency` (String) with `fi.currency.map(|c| c.to_string())`.
  - Exchange/MarketState strings: `.map(|e| e.to_string())`.
- Options
  - Replace usages of `contract.expiration` with `contract.expiration_at.unwrap_or_else(|| ...)`, or use `contract.expiration_date` for calendar-only logic.
  - New optional fields `last_trade_at` and `greeks` are available (greeks currently not populated from Yahoo v7).
- History
  - Replace `resp.unadjusted_close[i]` with `resp.candles[i].close_unadj.as_ref()`.

### Internal

- Tests updated for `httpmock` 0.8 API changes.
- Lints and examples adjusted for Money/typed enums.

## [0.3.2] - 2025-10-03

### Changed

- Bump `paft` to 0.3.2 (docs-only upstream release; no functional impact).

## [0.3.1] - 2025-10-02

### Changed

- Internal migration to `paft` 0.3.0 without changing the public API surface.
  - Switched internal imports to `paft::domain` (domain types) and `paft::money` (money/currency).
  - Updated internal `Money` construction to the new `Result`-returning API and replaced scalar ops with `try_mul` where appropriate.
- Examples and docs now import DataFrame traits from `paft::prelude::{ToDataFrame, ToDataFrameVec}`.
- Conversion helpers in `core::conversions` now document potential panics if a non-ISO currency lacks registered metadata (behavior aligned with `paft-money`).
- Profile ISIN fields now validate ISIN format using `paft::domain::Isin` - invalid ISINs are filtered out and stored as `None`.
- Updated tokio-tungstenite to version 0.28

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

[0.7.0]: https://github.com/gramistella/yfinance-rs/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/gramistella/yfinance-rs/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/gramistella/yfinance-rs/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/gramistella/yfinance-rs/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/gramistella/yfinance-rs/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/gramistella/yfinance-rs/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/gramistella/yfinance-rs/compare/v0.3.1...v0.4.0
[0.3.2]: https://github.com/gramistella/yfinance-rs/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/gramistella/yfinance-rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/gramistella/yfinance-rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/gramistella/yfinance-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/gramistella/yfinance-rs/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/gramistella/yfinance-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/gramistella/yfinance-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/gramistella/yfinance-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/gramistella/yfinance-rs/releases/tag/v0.1.0
