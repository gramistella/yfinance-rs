use futures::{StreamExt, stream};

use crate::{
    core::client::normalize_symbols,
    core::{
        CallOptions, Candle, Interval, ProjectionContext, ProjectionIssue, Range, YfClient,
        YfError, YfResponse, currency_resolver::ResolvedCurrencyUnit,
    },
    history::{HistoryBuilder, YahooHistoryResponse, round_candle_prices},
};
use paft::domain::{AssetKind, Instrument};
use paft::market::responses::{
    download::{DownloadEntry, DownloadResponse},
    history::{OhlcPriceBasis, PriceBasis},
};
type DateRange = (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>);
type MaybeDateRange = Option<DateRange>;
type DownloadFetchSuccess = (usize, String, YfResponse<YahooHistoryResponse>);
type DownloadFetchFailure = (usize, String, YfError);
type DownloadFetchResult = Result<DownloadFetchSuccess, DownloadFetchFailure>;
const UNTYPED_DOWNLOAD_ASSET_KIND: &str = "YAHOO_DOWNLOAD_UNTYPED";

/// Maximum number of per-symbol history requests a [`DownloadBuilder`] runs at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DownloadConcurrency(usize);

impl DownloadConcurrency {
    /// Default download concurrency.
    pub const DEFAULT: Self = Self(8);

    /// Builds a validated download concurrency limit.
    ///
    /// # Errors
    ///
    /// Returns `YfError::InvalidParams` if `value` is zero.
    pub fn new(value: usize) -> Result<Self, YfError> {
        if value == 0 {
            return Err(YfError::InvalidParams(
                "download concurrency must be at least 1".into(),
            ));
        }
        Ok(Self(value))
    }

    const fn get(self) -> usize {
        self.0
    }
}

impl Default for DownloadConcurrency {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Price adjustment mode for multi-symbol downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DownloadAdjustment {
    /// Preserve Yahoo's raw OHLC prices.
    None,
    /// Adjust OHLC prices for splits and dividends. This matches the download default.
    #[default]
    Auto,
    /// Adjust Open/High/Low for splits and dividends while preserving raw Close.
    Back,
}

impl DownloadAdjustment {
    const fn fetches_adjusted_history(self) -> bool {
        matches!(self, Self::Auto | Self::Back)
    }

    const fn is_back_adjusted(self) -> bool {
        matches!(self, Self::Back)
    }
}

/// A builder for downloading historical data for multiple symbols concurrently.
///
/// This provides a convenient way to fetch data for a list of tickers with the same
/// parameters in parallel, similar to `yfinance.download` in Python.
///
/// Many of the configuration methods mirror those on [`HistoryBuilder`].
///
/// When Yahoo omits `chart.meta.instrumentType`, best-effort downloads keep the
/// history entry using `AssetKind::Other("YAHOO_DOWNLOAD_UNTYPED")` and emit a
/// repair diagnostic. Strict mode treats that diagnostic as an error.
pub struct DownloadBuilder {
    client: YfClient,
    symbols: Vec<String>,

    // date / time controls
    range: Option<Range>,
    period: Option<(i64, i64)>,
    interval: Interval,

    // behavior flags
    adjustment: DownloadAdjustment,
    include_prepost: bool,
    include_actions: bool,
    rounding: bool,

    options: CallOptions,
    concurrency: DownloadConcurrency,
}

impl DownloadBuilder {
    fn precompute_period_dt(&self) -> Result<MaybeDateRange, YfError> {
        if let Some((p1, p2)) = self.period {
            use chrono::{TimeZone, Utc};
            let start = Utc
                .timestamp_opt(p1, 0)
                .single()
                .ok_or_else(|| YfError::InvalidParams("invalid period1".into()))?;
            let end = Utc
                .timestamp_opt(p2, 0)
                .single()
                .ok_or_else(|| YfError::InvalidParams("invalid period2".into()))?;
            if start >= end {
                return Err(YfError::InvalidDates);
            }
            Ok(Some((start, end)))
        } else {
            Ok(None)
        }
    }

    fn build_history_for_symbol(
        &self,
        sym: &str,
        period_dt: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
        need_adjust_in_fetch: bool,
    ) -> HistoryBuilder {
        let mut hb: HistoryBuilder = HistoryBuilder::new(&self.client, sym.to_string())
            .interval(self.interval)
            .auto_adjust(need_adjust_in_fetch)
            .prepost(self.include_prepost)
            .actions(self.include_actions)
            .data_quality(self.options.data_quality())
            .cache_mode(self.options.cache_mode())
            .retry_policy(self.options.retry_override().cloned());

        if let Some((start, end)) = period_dt {
            hb = hb.between(start, end);
        } else if let Some(r) = self.range {
            hb = hb.range(r);
        } else {
            hb = hb.range(Range::M6);
        }
        hb
    }

    fn apply_back_adjust(&self, rows: &mut [Candle]) {
        if !self.adjustment.is_back_adjusted() {
            return;
        }
        for c in rows.iter_mut() {
            if let Some(rc) = c.close_unadj.as_ref() {
                c.ohlc.close = rc.clone();
            }
        }
    }

    const fn back_adjust_price_basis(&self, fetched_basis: OhlcPriceBasis) -> OhlcPriceBasis {
        if !self.adjustment.is_back_adjusted() {
            return fetched_basis;
        }

        let (open, high, low, _) = fetched_basis.fields();
        OhlcPriceBasis::per_field(*open, *high, *low, PriceBasis::raw())
    }

    fn apply_rounding_if_enabled(
        &self,
        rows: &mut [Candle],
        price_hint: Option<u32>,
        currency_unit: Option<&ResolvedCurrencyUnit>,
    ) {
        if !self.rounding {
            return;
        }

        let Some(price_hint) = price_hint else {
            return;
        };

        round_candle_prices(rows, price_hint, currency_unit);
    }

    fn process_joined_results(
        &self,
        joined: Vec<(String, YfResponse<YahooHistoryResponse>)>,
        ctx: &mut ProjectionContext,
    ) -> Result<DownloadResponse, YfError> {
        let mut entries: Vec<DownloadEntry> = Vec::with_capacity(joined.len());
        for (sym, response) in joined {
            ctx.extend(response.diagnostics.with_key_prefix(&sym));
            let YahooHistoryResponse {
                response: mut resp,
                price_hint,
                currency_unit,
                instrument,
            } = response.data;
            // apply transforms to candles
            self.apply_back_adjust(&mut resp.candles);
            resp.price_basis = self.back_adjust_price_basis(resp.price_basis);
            self.apply_rounding_if_enabled(&mut resp.candles, price_hint, currency_unit.as_ref());

            let instrument = resolve_download_instrument(instrument, &sym, ctx)?;

            entries.push(DownloadEntry {
                instrument,
                history: resp,
                provider: (),
            });
        }
        Ok(DownloadResponse {
            entries,
            provider: (),
        })
    }

    /// Creates a new `DownloadBuilder`.
    #[must_use]
    pub fn new(client: &YfClient) -> Self {
        Self {
            client: client.clone(),
            symbols: Vec::new(),
            range: Some(Range::M6),
            period: None,
            interval: Interval::D1,
            adjustment: DownloadAdjustment::default(),
            include_prepost: false,
            include_actions: true,
            rounding: false,
            options: CallOptions::default(),
            concurrency: DownloadConcurrency::DEFAULT,
        }
    }

    crate::core::impl_call_option_setters!();

    /// Sets the maximum number of per-symbol history requests to run at once. (Default: `8`)
    #[must_use]
    pub const fn concurrency(mut self, concurrency: DownloadConcurrency) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Replaces the current list of symbols with a new list.
    #[must_use]
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(std::convert::Into::into).collect();
        self
    }

    /// Adds a single symbol to the list of symbols to download.
    #[must_use]
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }

    /// Sets a relative time range for the request (e.g., `1y`, `6mo`).
    #[must_use]
    pub const fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }

    /// Sets an absolute time period for the request using start and end timestamps.
    #[must_use]
    pub const fn between(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.range = None;
        self.period = Some((start.timestamp(), end.timestamp()));
        self
    }

    /// Sets the time interval for each data point (candle).
    #[must_use]
    pub const fn interval(mut self, interval: Interval) -> Self {
        self.interval = interval;
        self
    }

    /// Sets the price adjustment mode. (Default: [`DownloadAdjustment::Auto`])
    #[must_use]
    pub const fn adjustment(mut self, adjustment: DownloadAdjustment) -> Self {
        self.adjustment = adjustment;
        self
    }

    /// Preserves Yahoo's raw OHLC prices.
    #[must_use]
    pub const fn unadjusted(mut self) -> Self {
        self.adjustment = DownloadAdjustment::None;
        self
    }

    /// Adjusts OHLC prices for splits and dividends. This is the default mode.
    #[must_use]
    pub const fn auto_adjust(mut self) -> Self {
        self.adjustment = DownloadAdjustment::Auto;
        self
    }

    /// Back-adjusts prices.
    ///
    /// Back-adjustment adjusts the Open, High, and Low prices, but keeps the Close price as raw
    /// Yahoo close.
    #[must_use]
    pub const fn back_adjust(mut self) -> Self {
        self.adjustment = DownloadAdjustment::Back;
        self
    }

    /// Sets whether to include pre-market and post-market data for intraday intervals. (Default: `false`)
    #[must_use]
    pub const fn prepost(mut self, yes: bool) -> Self {
        self.include_prepost = yes;
        self
    }

    /// Sets whether to include corporate actions (dividends and splits) in the result. (Default: `true`)
    #[must_use]
    pub const fn actions(mut self, yes: bool) -> Self {
        self.include_actions = yes;
        self
    }

    /// Sets whether to round prices using Yahoo's chart `priceHint`. (Default: `false`)
    ///
    /// This rounds candle price fields after adjustment. Corporate-action amounts are left exact.
    #[must_use]
    pub const fn rounding(mut self, yes: bool) -> Self {
        self.rounding = yes;
        self
    }

    /// Executes the download by fetching data for all specified symbols concurrently.
    ///
    /// # Errors
    ///
    /// Returns an error if parameters are invalid or strict data-quality mode rejects a
    /// diagnostic. In best-effort mode, individual symbol fetch failures are returned as
    /// diagnostics while successful symbols are still included in the response.
    pub async fn run(&self) -> Result<DownloadResponse, YfError> {
        Ok(self.run_with_diagnostics().await?.into_data())
    }

    /// Executes the download and returns projection diagnostics.
    ///
    /// # Errors
    ///
    /// Returns an error if parameters are invalid or strict data-quality mode rejects a
    /// diagnostic. In best-effort mode, individual symbol fetch failures are returned as
    /// diagnostics while successful symbols are still included in the response.
    pub async fn run_with_diagnostics(&self) -> Result<YfResponse<DownloadResponse>, YfError> {
        if self.symbols.is_empty() {
            return Err(YfError::InvalidParams("no symbols specified".into()));
        }
        let symbols = normalize_symbols(self.symbols.iter().map(String::as_str))?;
        let mut ctx = ProjectionContext::new("download", self.options.data_quality());

        let need_adjust_in_fetch = self.adjustment.fetches_adjusted_history();
        let period_dt = self.precompute_period_dt()?;

        let results: Vec<DownloadFetchResult> = stream::iter(symbols.into_iter().enumerate())
            .map(|(index, sym)| {
                let hb = self.build_history_for_symbol(&sym, period_dt, need_adjust_in_fetch);

                async move {
                    match hb.fetch_full_yahoo_with_diagnostics().await {
                        Ok(full) => Ok((index, sym, full)),
                        Err(err) => Err((index, sym, err)),
                    }
                }
            })
            .buffer_unordered(self.concurrency.get())
            .collect()
            .await;

        let mut joined = Vec::with_capacity(results.len());
        let mut failed = Vec::new();
        for result in results {
            match result {
                Ok(success) => joined.push(success),
                Err(failure) => failed.push(failure),
            }
        }

        failed.sort_unstable_by_key(|(index, _, _)| *index);
        for (_, sym, err) in failed {
            ctx.dropped_item(
                "download_entry",
                Some(sym.as_str()),
                ProjectionIssue::ProviderError {
                    message: format!("history fetch failed: {err}"),
                },
            )?;
        }

        joined.sort_unstable_by_key(|(index, _, _)| *index);
        let joined: Vec<(String, YfResponse<YahooHistoryResponse>)> = joined
            .into_iter()
            .map(|(_, sym, full)| (sym, full))
            .collect();

        let response = self.process_joined_results(joined, &mut ctx)?;
        Ok(ctx.finish(response))
    }
}

/* ---------------- internal helpers ---------------- */

fn resolve_download_instrument(
    instrument: Option<Instrument>,
    symbol: &str,
    ctx: &mut ProjectionContext,
) -> Result<Instrument, YfError> {
    if let Some(instrument) = instrument {
        return Ok(instrument);
    }

    ctx.repaired_data(
        "download_entry",
        Some(symbol),
        "used untyped Yahoo download instrument because chart.meta.instrumentType was missing",
    )?;

    Instrument::from_symbol(symbol, untyped_download_asset_kind()).map_err(|err| {
        YfError::InvalidParams(format!("invalid download symbol fallback {symbol}: {err}"))
    })
}

fn untyped_download_asset_kind() -> AssetKind {
    AssetKind::other(UNTYPED_DOWNLOAD_ASSET_KIND).expect("valid download fallback asset kind")
}
