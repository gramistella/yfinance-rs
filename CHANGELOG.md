# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.1] - 2026-07-11

### Fixed

- Maximum-range history requests now use explicit, interval-aware Yahoo date
  windows instead of `range=max`, preventing Yahoo from silently returning
  quarterly candles for a requested daily interval. History responses also
  reject a present `dataGranularity` that does not match the requested interval.
- The Polars dataframe example's analysis queries now use the `paft` 0.9
  `volume.amount` history column name instead of the removed `volume` column.

## [0.9.0] - 2026-06-10

### Breaking Changes

- Declared Rust 1.91 as the crate's minimum supported Rust version through
  package metadata.
- `QuotesBuilder::new` now borrows `&YfClient`, matching the other public
  builder constructors. Public fetch/run/start methods now borrow the configured
  builder instead of consuming it.
- `StreamBuilder::start()` is now async. In `StreamMethod::Websocket` mode it
  waits for the initial WebSocket handshake and subscription write, returning
  startup failures directly.
- The `stream` module and crate-root stream builder re-exports are now behind
  the opt-in `stream` feature.
- `HistoryService::fetch_full_history()` now returns an unboxed
  `impl Future + Send` instead of a pinned boxed future; custom trait
  implementors must update their signatures.
- `YfClient::clear_cache()` and `YfClient::invalidate_cache_entry()` are now
  synchronous. `clear_cache()` also clears response, currency-hint,
  resolved-currency, and instrument caches; `invalidate_cache_entry()` remains
  response-cache only.
- `CacheMode` now has a policy-driven `Default` mode. Volatile endpoints such
  as quotes, options, news, and screeners bypass response caching by default;
  use `CacheMode::Use` to opt them into caching.
- Removed the unused `yfinance_rs::core::dataframe::ToDataFrame` trait. The
  `dataframe` feature now re-exports `paft` dataframe traits at
  `yfinance_rs::{dataframe, ToDataFrame, ToDataFrameVec}`.
- Removed `HistoryRequest::keepna`, `HistoryBuilder::keepna`, and
  `DownloadBuilder::keepna`; malformed history OHLC rows are dropped with
  diagnostics instead of fabricating placeholder prices.
- `DownloadBuilder` now models price adjustment with `DownloadAdjustment`.
  The old `auto_adjust(bool)` and `back_adjust(bool)` setters are replaced by
  `.adjustment(...)`, `.auto_adjust()`, `.back_adjust()`, and `.unadjusted()`.
- Removed `DownloadBuilder::repair()` and the download price-outlier repair
  heuristic; downloads no longer apply the 100x outlier repair pass.
- History responses now use `paft`'s `price_basis` metadata instead of the old
  `adjusted` boolean. Back-adjusted downloads report adjusted open/high/low and
  raw close as per-field OHLC bases.
- Removed tuple-based `Ticker::dividends()`, `Ticker::splits()`, and
  `Ticker::capital_gains()` helpers. Use `Ticker::actions()` and match on typed
  `Action` variants instead.
- Removed `Info::esg_scores`; `Ticker::info()` no longer fetches the
  `esgScores` module. Use `Ticker::sustainability()` for explicit ESG requests.
- Removed the legacy profile HTML scraping fallback and its configuration
  surface: `YfClientBuilder::base_quote()`, public `ApiPreference`, hidden
  `YfClientBuilder::_api_preference()`, and profile source-selection test paths.
  Profiles now load only from Yahoo's quoteSummary API.
- `Ticker::fast_info()` now returns yfinance-rs' own `FastInfo` struct with
  instant quote data under `snapshot` and moving averages under
  `moving_averages`.
- The public model now follows the `paft` 0.9 shape: quote, snapshot, stream,
  history, option premium (`price`/`bid`/`ask`), and book-level price fields
  use currency-less `PriceAmount` values with currency carried by the
  containing model; stream, quote, and history volumes plus book-level sizes
  use `QuantityAmount`; `Candle` prices live under `candle.ohlc`; actions and
  share/calendar dates are calendar dates; and `ReportingPeriod` replaces the
  old period type.
- `QuoteUpdate::volume` now exposes Yahoo's latest cumulative session volume as
  a `QuantityAmount` instead of a computed per-update delta.
- `ScreenerNumber` is now an opaque validated value. Floating-point values must
  be constructed with `ScreenerNumber::new`; integer values still use the
  existing `From` conversions.
- `YfError` has new variants for provider-data, data-quality, retry, money,
  and option-chain failures: `InvalidData`, `DataQuality`,
  `RequestNotCloneable`, `Money`, and `OptionUnderlyingTypeUnavailable`.
- `YfError::Http` now stores a redacted HTTP-client error wrapper instead of
  `reqwest::Error`, preventing formatted errors from leaking auth-like query
  parameters.
- `YfError` JSON, WebSocket, Protobuf, Base64, and URL variants now store
  yfinance-rs-owned opaque wrapper types instead of directly exposing foreign
  parser and transport error types. The original errors remain available through
  `std::error::Error::source()`.
- Replaced the old lossy float-to-money/price/decimal helpers with checked
  conversion helpers. Doc-hidden test/plumbing APIs including
  `core::conversions`, `core::yahoo_vocab`, `YfClientBuilder::_preauth()`, and
  `stream::decode_and_map_message()` are now crate-private or unavailable in
  normal builds and public only for unit tests or the `test-mode` feature.
- Removed the panicking `YfClientBuilder::proxy()` and
  `YfClientBuilder::https_proxy()` setters. Use `try_proxy()` and
  `try_https_proxy()` instead.
- Removed `SearchBuilder::news_count()` and `SearchBuilder::lists_count()`;
  search responses currently expose quote results only, so the builder no
  longer advertises unsupported result surfaces.
- Projection-aware parsers now reject, diagnose, or drop many malformed provider
  classification, currency, date, and numeric fields instead of silently using
  defaults such as USD, epoch timestamps, `Equity`, or zero-valued financial
  data.
- Existing growable public enums such as `YfError`, `NewsTab`, and several
  Yahoo screener vocabularies are now `#[non_exhaustive]`; open-ended
  diagnostics enums such as `YfWarning`, `ProjectionIssue`,
  `YfCurrencyPurpose`, and `YfCurrencyInference` use the same policy.
- Marked growable public response and diagnostics structs as
  `#[non_exhaustive]`, including `Info`, `FastInfo`, `MovingAverages`,
  `ScreenerResponse`, `ScreenerResult`, `YfResponse<T>`, and `YfDiagnostics`.

### Added

- Added adapter-level projection diagnostics through `YfResponse<T>`,
  `YfDiagnostics`, `YfWarning`, `ProjectionIssue`, and `DataQuality`.
- Added diagnostic-returning entry points for quotes, fast info, key statistics,
  option chains, screeners, history, download, holders, fundamentals, analysis,
  ESG, news, profile, search, and aggregate ticker info.
- Added strict projection controls through builder `data_quality()`/`strict()`
  methods, `Ticker::data_quality()`, `Ticker::strict()`, and
  `Ticker::info_strict()`.
- Added currency-inference diagnostics through `YfCurrencyPurpose` and
  `YfCurrencyInference`, plus `YfWarning::CoercedPresentField` for lossy
  coercions.
- Added cache tuning through public `CacheEndpoint` buckets,
  `YfClientBuilder::cache_ttl_for()`,
  `YfClientBuilder::cache_max_entries()`, and
  `YfClientBuilder::side_cache_max_entries()` for internal side caches.
- Added `StreamBuilder::websocket_connect_timeout()` and
  `StreamBuilder::websocket_idle_timeout()`.
- Added `DownloadConcurrency` and `DownloadBuilder::concurrency()`; downloads
  run at most 8 per-symbol history requests concurrently by default.
- Added explicit share-count windows through `Ticker::shares_between()`,
  `Ticker::quarterly_shares_between()`, and
  `FundamentalsBuilder::shares_between()` and
  `FundamentalsBuilder::shares_between_with_diagnostics()`.
- Added `FastInfo::moving_averages` and `Info::moving_averages` for Yahoo's
  50-day and 200-day average prices.
- Added `Ticker::profile()` as a high-level convenience method for company,
  ETF, and mutual-fund profiles.
- Re-exported common `paft` decimal, money, currency, domain, market,
  fundamentals, options, news, download, and search model types from the crate
  root.
- Re-exported `Backoff`, `MovingAverages`, `HistoryRequest`,
  `HistoryService`, `ScreenerQuery`, and `YahooQuoteType` from the crate root.
- Added `quotes_with_diagnostics()` and `screen_with_diagnostics()` crate-root
  helpers.
- Added high-level `Ticker::*_with_diagnostics()` convenience methods for
  news, history, actions, holders, analysis, fundamentals, and share counts.
- Re-exported `AnalysisBuilder` from the crate root for consistency with the
  other endpoint builders.
- Added `examples/16_diagnostics_audit.rs`, a live public-surface audit that
  demonstrates checking projection diagnostics across the crate.

### Dependencies

- Added `serde_field_result` 0.1.0 from crates.io for recoverable provider
  wire-field deserialization.
- Added `moka` for bounded in-memory caches and `getrandom` for retry jitter.
- Added `unicode-normalization` for country/currency inference normalization.
- Removed protobuf code generation from crate builds. The tiny generated
  Yahoo stream wire module is now committed, and stream-only dependencies
  (`base64`, `prost`, `futures-util`, and `tokio-tungstenite`) are enabled only
  by the `stream` feature.
- Bumped `paft` from crates.io `0.8.0` to `0.9.0`.
- Switched reqwest from `rustls-tls` to `rustls-tls-native-roots`, aligning HTTP
  TLS root handling with the existing WebSocket native-root setup.
- Disabled reqwest's `cookies` feature; `YfClient` handles Yahoo auth cookies
  explicitly without reqwest's cookie store.
- Removed inert `[package.metadata.cargo-doc]`; docs.rs build configuration now
  lives only under `[package.metadata.docs.rs]`.
- Removed the direct optional/runtime `polars` dependency from `yfinance-rs`;
  the `dataframe` feature now enables `paft/dataframe`, while `polars` remains
  only as a dev-dependency for examples and tests.
- Enabled the direct Tokio `sync` and `time` features used by the crate.
- Removed the direct `thiserror` dependency; `YfError` now uses manual
  `Display` and `Error` implementations to support redacted and opaque error
  wrappers.

### Fixed

- Published crate packages now exclude repo-only integration tests and recorded
  Yahoo fixture payloads under `tests/`.
- Download rounding now uses Yahoo chart `priceHint` metadata instead of
  hardcoded two-decimal `f64` rounding.
- Download rounding now applies Yahoo chart `priceHint` before converting
  Yahoo subunit quote currencies such as `GBp` into major currency amounts.
- Sparse Yahoo `adjclose` history payloads now use one coherent split-only
  adjustment basis with diagnostics instead of mixing adjustment bases by row.
- Predefined screener GET responses are now parsed before response-cache writes,
  preventing successful-status Yahoo error or malformed bodies from being cached.
- Provider-adjusted history candle factors are now computed while validating
  `adjclose` coverage, avoiding a fragile cross-function assembly invariant.
- Downloads now keep successful history entries when Yahoo omits
  `chart.meta.instrumentType`, using an explicit untyped instrument fallback
  with diagnostics instead of dropping the symbol in best-effort mode.
- Multi-symbol downloads now carry each chart response's resolved instrument
  through assembly, avoiding false untyped fallbacks or strict-mode failures
  when the bounded side cache evicts earlier history metadata.
- v7 quote side effects now reuse already-projected quote nodes instead of
  cloning and re-deserializing raw JSON values.
- Batch v7 quote fetches now split large symbol lists into bounded URL-size
  chunks and fetch those chunks with bounded concurrency.
- Chart responses that omit `timestamp` while carrying quote or `adjclose` data
  now fail as malformed; timestamp-less empty quote and `adjclose` payloads
  continue to decode as empty history results.
- `_preauth` now seeds client credentials in crate unit-test builds (`cfg(test)`)
  as well as when the `test-mode` feature is enabled.
- Business Insider ISIN lookup now parses the `mmSuggestDeliver` JSONP shape
  with a local data-expression parser, returns typed HTTP status errors for
  non-success responses, validates ISIN check digits, and keeps suffix-qualified
  symbols distinct while matching suggestions.
- Stock split action ratios now normalize Yahoo split components with exact
  decimal arithmetic instead of f64 scaling and rounding.
- History candle assembly now validates timestamps and raw OHLC values before
  adjustment, drops malformed rows with diagnostics, and preallocates from the
  shortest required OHLC/timestamp array.
- History now resolves candle and action currencies through source-aware trading
  and corporate-action resolvers; dividend and capital-gain events can use
  event-level currencies, then chart/trading defaults, before inferred
  fallbacks.
- History best-effort responses now report unresolved candle or action currency
  as dropped-item diagnostics instead of aborting the whole response or falling
  back to USD.
- History requests now ignore malformed `chart.meta.instrumentType` values when
  the field is only needed for side-cache enrichment, preserving otherwise
  valid candles.
- `DownloadBuilder` best-effort batches now preserve successful symbols when an
  individual history fetch fails and report failed symbols as dropped entries.
- `DownloadBuilder` now validates explicit date ranges once before spawning
  per-symbol history requests, preserving top-level `YfError::InvalidDates`
  while best-effort mode reports other per-symbol fetch failures as diagnostics.
- Download rounding now leaves values unchanged when conversion fails instead
  of falling back to zero.
- `Ticker::info()` now projects profile, key-statistics, price-target,
  recommendation-summary, and calendar data from one quoteSummary response,
  avoiding duplicate `financialData` fetches.
- `Ticker::key_statistics()` and `info.key_statistics` now backfill additional
  quoteSummary valuation, dividend, range, beta, and volume fields when Yahoo's
  v7 quote response omits them.
- Profile loading now supports Yahoo `MUTUALFUND` quote types and ETF profiles
  whose `fundProfile.legalType` is absent; unsupported profile quote types such
  as indexes and cryptocurrencies now return provider-data errors.
- QuoteSummary-backed analysis, earnings, calendar, and profile projection now
  tolerate wrong-type optional fields in best-effort mode and report diagnostics
  instead of failing the whole endpoint with `YfError::Json`.
- Missing optional quoteSummary modules, including analysis, ESG, calendar, and
  holder modules, now emit `ProviderFeatureUnavailable` diagnostics in
  best-effort mode and data-quality errors in strict mode.
- Best-effort projection now skips malformed required records item-by-item for
  history candles/actions, quote nodes, option contracts, holder rows, search
  results, screener results, news articles, and fundamentals timeseries values.
- Best-effort projection now omits malformed optional fields with diagnostics
  while preserving otherwise usable sibling data; strict mode rejects those
  projection losses.
- Yahoo v7 quote, options, and fundamentals-timeseries payload errors now
  surface as `YfError::Api` or typed status errors before being treated as
  missing data.
- Batch quotes with diagnostics now report requested symbols that Yahoo omits
  from the v7 response instead of silently returning a shorter quote vector.
- Yahoo counter fields such as quote volume/book sizes, screener volume, and
  option contract volume/open interest now accept numeric strings.
- Search requests now reject empty or whitespace-only queries with
  `YfError::InvalidParams` before contacting Yahoo.
- Public retry policies, stream intervals/timeouts, and user-provided symbols
  are now validated before use, returning `YfError::InvalidParams` instead of
  panicking or issuing malformed Yahoo requests.
- The Polars dataframe and convenience-methods examples now raise their crate
  recursion limits so all-target clippy/check builds compile under current
  stable Rust.
- Yahoo exchange and quote-type vocabulary is now normalized through one shared
  adapter across quote, fast info, info, search, screener, history, options,
  stream, and currency-inference paths.
- Currency resolution is now source-aware and purpose-aware across trading,
  reporting, corporate-action, and analyst-estimate values; invalid provider
  currencies, failed enrichment, and unresolved heuristics now produce
  diagnostics or data errors instead of silent USD fallbacks.
- Yahoo quote-unit currency codes such as `GBp`, `GBX`, `ZAc`, and `ILA` are
  normalized to their major ISO currencies; per-share prices are scaled from
  quote units while aggregate money values stay in major units.
- v7 quote key statistics, holder values, insider transaction values, and
  fundamentals statements now distinguish quote-unit, trading-currency, and
  reporting-currency values more precisely.
- Fundamentals timeseries statements now use same-payload `currencyCode` values
  before quote/profile enrichment, parse large wire numbers directly into
  decimals, process every flattened field in grouped Yahoo result objects, and
  skip present-but-empty `reportedValue` wrappers.
- Historical share-count helpers now request Yahoo's annual/quarterly
  `OrdinarySharesNumber` timeseries instead of basic-average-shares fields, and
  default share-count windows now keep response-cache keys stable within a day.
- Earnings trend, analyst estimate, recommendation, calendar, holder, ESG, and
  upgrade/downgrade mappers now route present-but-unrepresentable periods,
  dates, decimals, currencies, and grade/action fields through projection
  diagnostics.
- Upgrade/downgrade rows now report invalid present analyst firm fields as
  omitted-field diagnostics instead of silently treating them as missing.
- Recommendation summaries now populate `mean_rating_text` from Yahoo's
  `recommendationKey`.
- Option endpoints now surface Yahoo `optionChain.error` payloads as
  `YfError::Api`; option-chain projection now parses contracts item-by-item and
  reports missing contract currency or invalid strikes through diagnostics.
- Option chains with contracts but no usable Yahoo underlying `quoteType` now
  fail with `YfError::OptionUnderlyingTypeUnavailable`; empty chains no longer
  require typed underlying metadata.
- Option chains now resolve missing contract currency through the trading
  currency resolver instead of depending on already-converted quote prices.
- WebSocket streams now use the configured reqwest client for the startup
  upgrade, so builder and custom-client proxy/DNS/TLS configuration is honored.
- WebSocket fallback mode now treats idle sockets, remote close/EOF, and startup
  failures as recoverable stream failures and retries WebSocket connections
  after fallback polling.
- Polling streams now observe stop requests while a quote HTTP request is in
  flight and timestamp quote updates with Yahoo's `regularMarketTime` when
  available.
- Streaming quote updates now pass Yahoo cumulative volume through directly,
  apply Yahoo `price_hint`/direct f32 decimal conversion for WebSocket prices,
  preserve equity prices when Yahoo omits stream currency but sends exchange
  metadata, and avoid poisoning the instrument cache with untyped fallbacks.
- Polling streams in `diff_only` mode now emit only on price changes;
  volume-only cumulative-volume changes no longer trigger an update.
- General proxy configuration through `YfClientBuilder::try_proxy()` now applies
  to HTTPS requests instead of only plain HTTP URLs.
- `YfClient::default()` and builder-created clients that do not use
  `custom_client()` now apply a 30-second total request timeout and a 10-second
  connect timeout by default.
- Yahoo crumb-auth retries are centralized across crumb-authenticated endpoints:
  stale crumbs are cleared on 401/403 or invalid-crumb bodies, cached invalid
  responses are evicted, and fresh-credential retries bypass stale cache reads.
- Optional crumb endpoints now use an already cached crumb before trying a bare
  request, avoiding repeated auth failures on Yahoo surfaces that require crumbs.
- Yahoo auth now sends the stored cookie explicitly during crumb acquisition and
  crumb-authenticated requests, so custom reqwest clients no longer need
  reqwest's cookie store enabled.
- Builder-created clients no longer enable reqwest's ambient cookie store, and
  crumb acquisition now rejects non-success statuses, error-shaped crumb bodies,
  whitespace/control-bearing values, and suspiciously long values before caching
  a trimmed crumb.
- Response-cache keys for POST endpoints with `cache_mode(CacheMode::Use)`,
  including news and custom screeners, now include the serialized request body.
- Cached responses for endpoints with provider-error validators (chart, v7
  quote, quoteSummary, options, fundamentals-timeseries, search, and screeners)
  are now checked before cache writes; stale cached bodies that fail the same
  validation are evicted instead of replayed.
- Response cache now uses bounded `moka`; side caches are bounded in-memory
  maps; stale response-cache entries are removed on access and expired entries
  are pruned on writes.
- Status errors, HTTP-client errors, WebSocket startup status errors, tracing
  URL fields, and `YfClient` debug output now redact crumb and auth-like query
  parameters.
- Yahoo symbol path URLs are now built with one percent-encoding helper instead
  of `Url::join`, preventing symbols containing URL syntax from changing the
  request target.
- Exponential retry backoff now uses random jitter when configured and validates
  retry policies before use.
- `Ticker`-level cache, retry, and data-quality settings now propagate through
  builders, action helpers, and profile loading inside `Ticker::info()`.
- Holder convenience methods now request only the quoteSummary module they
  project instead of fetching every holder/insider module for each call.

### Changed

- `None` currency overrides continue to auto-enrich by querying Yahoo for
  stronger currency evidence when an endpoint omits currency data.
  `Some(currency)` overrides remain per-call only and no longer mutate inferred
  currency caches or emit `CurrencyInferred` diagnostics.
- Provider-backed quote, quoteSummary, direct, override, and cached currency
  evidence no longer emits `CurrencyInferred` diagnostics; only listing and
  profile-country heuristics are diagnostic inferred currency fallbacks.
- Profile-country currency inference now uses normalized country aliases for
  exact and fuzzy lookups, including punctuated names such as `Timor-Leste`.
- Country-based currency inference now uses Unicode normalization to strip
  combining diacritics.
- Examples and README snippets were updated for the `paft` 0.9 model,
  diagnostics, fallible ESG data, and the removal of public conversion-helper
  usage.
- Documented `test-mode`, `debug-dumps`, and `tracing-subscriber` as
  internal/unstable Cargo features rather than user-facing options.
- docs.rs builds now enable only the public optional `stream`, `dataframe`, and
  `tracing` features instead of `all-features`, keeping internal maintenance
  features out of published API docs.
- Published crate packages now exclude repository workflow metadata and tracked
  macOS editor artifacts.

## [0.8.0] - 2026-05-27

### Breaking Changes

- Bump the crate to `0.8.0` and move the public API onto the `paft` 0.8 model. This is a breaking release for consumers that construct or destructure exported `paft` types directly.
- Per-unit quoted values now use `paft::money::Price` where `paft` 0.8 does so. This affects quotes, snapshots/fast-info, history candles, option prices and strikes, analyst EPS/price-target values, and action dividend/capital-gain amounts. Settled totals, such as market cap, remain `Money`.
- `Ticker::info()` now returns a composed, structured `Info` rather than a flat market/fundamentals struct. Market snapshot fields live under `info.snapshot`, statistics live under `info.key_statistics`, and optional modules live under `info.profile`, `info.calendar`, `info.price_target`, `info.recommendation_summary`, and `info.esg_scores`.
- Quote, search, option, and stream payloads now prefer `paft::domain::Instrument` over raw symbol fields where the updated `paft` model does so. Download entries already carried instruments, but the updated `paft` model changes access to public fields such as `entry.instrument.symbol`.
- Historical split actions now expose non-zero split ratios through `std::num::NonZeroU32` numerator and denominator fields.
- `Ticker::fast_info()` returns `paft::aggregates::Snapshot`, keeping it strictly scoped to instant-in-time quote data (`last`, `previous_close`, `open`, `day_high`, `day_low`, `volume`, and market state). Exchange identity now lives on `snapshot.instrument.exchange`, and currency is carried by the price fields.

### Added

- Add strongly typed Yahoo Finance screeners:
  - `PredefinedScreener`, `screen`, and `ScreenerBuilder` for predefined Yahoo screeners;
  - `EquityQuery`, `FundQuery`, and `EtfQuery` for custom typed screener queries;
  - closed field/value vocabularies, typed operators, bounded count/offset values, finite numeric values, and percent-point filters;
  - `ScreenerResponse` and `ScreenerResult` with paft identity parsing where possible and preserved Yahoo-specific extra fields.
- Add `Ticker::key_statistics()` and re-export `KeyStatistics` from the crate root.
- Re-export common `paft::domain` types from the crate root: `AssetKind`, `Exchange`,
  `Instrument`, `MarketState`, `Period`, and `Symbol`.
- Map more Yahoo v7 quote fields into provider-agnostic `paft` models: bid/ask top-of-book levels, regular-market open/high/low/time, market cap, shares outstanding, trailing EPS, trailing PE, dividend rate/yields, 52-week high/low, and three-month average volume.
- Expand financial statement mappings from Yahoo fundamentals-timeseries:
  - income statement: interest expense, income tax expense, depreciation and amortization;
  - balance sheet: current assets/liabilities, accounts receivable, inventory, accounts payable, net PPE, goodwill, and intangible assets excluding goodwill;
  - cash flow: depreciation and amortization.
- Add tests covering the new quote, key-statistics, fast-info, statement, and DataFrame conversion paths.

### Changed

- Align `Ticker::info()` more closely with Python yfinance's broad `info` intent while keeping the output grouped by `paft`'s provider-agnostic models instead of mirroring Yahoo's raw response shape.
- Treat optional `info()` submodules as best-effort: a valid v7 quote remains the required core, while profile, calendar, analyst, and ESG modules are included when available.
- Carry the existing calendar dividend-date semantics into the new composed info/key-statistics model: Yahoo `calendarEvents.exDividendDate` maps to `Calendar.ex_dividend_date`, Yahoo `calendarEvents.dividendDate` maps to `Calendar.dividend_payment_date`, and v7 `dividendDate` is used only as a fallback payment date. `KeyStatistics.ex_dividend_date` remains unset unless the upstream data is actually an ex-dividend date.
- Convert ratio and percentage-like analysis, ESG, holder, and quote statistics to `paft::Decimal` where the updated `paft` API expects decimal values.
- Extend range and interval conversion support for the additional variants provided by the updated `paft` market request model.
- Map options into the `paft` 0.8 option model with `OptionContractKey`, `OptionSide`, `contract_instrument`, and a single `contracts` list exposed through `calls()` and `puts()` iterators.

### Fixed

- Stop split-adjusting Yahoo chart volumes during historical `auto_adjust`; Python yfinance adjusts OHLC prices but leaves reported volume unchanged. This avoids double-adjusting already split-adjusted volumes around events such as NVDA's 2024-06-10 10:1 split.
- Tolerate fractional Yahoo split numerator/denominator values in history responses by normalizing them into gcd-simplified non-zero split actions. Oversized normalized pairs are skipped instead of aborting the whole history response.
- Emit the current cumulative day volume as the first stream volume delta after a detected reset/rollover, instead of emitting `None`; the first observed tick and the stateless WebSocket decoder still emit `None`.
- Prefer Yahoo long names over short names for search results, quotes, and fast-info snapshots
  when both names are available.
- Preserve option-chain underlying identity from Yahoo's options response metadata, including ETF/fund quote types and exchange context, instead of falling back to an equity-only request-symbol instrument.
- Populate beta in `Ticker::key_statistics()` and `info.key_statistics` from quoteSummary `summaryDetail`/`defaultKeyStatistics` when the v7 quote response does not include beta.

### Dependencies

- Bump `paft` from crates.io `0.7.1` to crates.io `0.8.0`.
- Bump Polars support to `0.53`.
- Set the `chrono` dependency to `0.4.41` for compatibility with the updated dependency graph.

### Migration Notes

- Replace flat `info` reads with the appropriate group, for example `info.symbol` to `info.snapshot.instrument.symbol`, `info.exchange` to `info.snapshot.instrument.exchange`, `info.last` to `info.snapshot.last`, `info.volume` to `info.snapshot.volume`, and `info.market_cap` to `info.key_statistics.market_cap`.
- Read dividend payment dates from `info.calendar.as_ref().and_then(|c| c.dividend_payment_date)` and ex-dividend dates from `info.calendar.as_ref().and_then(|c| c.ex_dividend_date)`.
- New `quote.bid` and `quote.ask` fields are `Option<BookLevel>`; read prices through `level.price` and sizes through `level.size`.
- Access instrument symbols through public fields, for example `quote.instrument.symbol.as_str()`, `search_result.instrument.symbol.as_str()`, `download_entry.instrument.symbol.as_str()`, and `update.instrument.symbol.as_str()`.
- Read split action ratios with `numerator.get()` and `denominator.get()` after matching `Action::Split`.
- Option chains now expose `contracts` plus `calls()`/`puts()` iterators. Contract economic identity lives under `contract.key`, while Yahoo's contract symbol is available through `contract.contract_instrument.as_ref().map(|i| i.symbol.as_str())`.

## [0.7.2] - 2025-10-31

### Dependencies

- Bump `paft` to `v0.7.1`.

### Note

Yahoo Finance appears to have removed or relocated the ESG data endpoint. As a result, `ticker.sustainability()` currently panics during normal usage and live testing. This issue is under investigation.

## [0.7.1] - 2025-10-30

### Fixed

- Format fundamentals timeseries statement row period from epoch to YYYY-MM-DD.
- Correct `calendarEvents` mapping and extraction for `exDividendDate` and `dividendDate`.
- Correct gross profit and operating income in income statement.

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

[Unreleased]: https://github.com/gramistella/yfinance-rs/compare/v0.9.1...HEAD
[0.9.1]: https://github.com/gramistella/yfinance-rs/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/gramistella/yfinance-rs/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/gramistella/yfinance-rs/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/gramistella/yfinance-rs/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/gramistella/yfinance-rs/compare/v0.7.0...v0.7.1
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
