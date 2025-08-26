mod actions;
mod adjust;
mod assemble;
mod fetch;

use crate::core::client::{CacheMode, RetryConfig};
use crate::core::models::{Action, Candle, HistoryMeta, HistoryResponse};
use crate::core::{Interval, Range, YfClient, YfError};
use crate::history::wire::MetaNode;

use actions::extract_actions;
use adjust::cumulative_split_after;
use assemble::assemble_candles;
use fetch::fetch_chart;

/// A builder for fetching historical price data for a single symbol.
///
/// This builder provides fine-grained control over the parameters for a historical
/// data request, including the time range, interval, and data adjustments.
#[derive(Clone)]
pub struct HistoryBuilder {
    #[doc(hidden)]
    pub(crate) client: YfClient,
    #[doc(hidden)]
    pub(crate) symbol: String,
    #[doc(hidden)]
    pub(crate) range: Option<Range>,
    #[doc(hidden)]
    pub(crate) period: Option<(i64, i64)>,
    #[doc(hidden)]
    pub(crate) interval: Interval,
    #[doc(hidden)]
    pub(crate) auto_adjust: bool,
    #[doc(hidden)]
    pub(crate) include_prepost: bool,
    #[doc(hidden)]
    pub(crate) include_actions: bool,
    #[doc(hidden)]
    pub(crate) keepna: bool,
    #[doc(hidden)]
    pub(crate) cache_mode: CacheMode,
    #[doc(hidden)]
    pub(crate) retry_override: Option<RetryConfig>,
}

impl HistoryBuilder {
    /// Creates a new `HistoryBuilder` for a given symbol.
    pub fn new(client: &YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client: client.clone(),
            symbol: symbol.into(),
            range: Some(Range::M6),
            period: None,
            interval: Interval::D1,
            auto_adjust: true,
            include_prepost: false,
            include_actions: true,
            keepna: false,
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for this specific API call.
    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call.
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Sets a relative time range for the request (e.g., `1y`, `6mo`).
    ///
    /// This will override any previously set period using `between()`.
    pub fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }

    /// Sets an absolute time period for the request using start and end timestamps.
    ///
    /// This will override any previously set range using `range()`.
    pub fn between(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.range = None;
        self.period = Some((start.timestamp(), end.timestamp()));
        self
    }

    /// Sets the time interval for each data point (candle).
    pub fn interval(mut self, interval: Interval) -> Self {
        self.interval = interval;
        self
    }

    /// Sets whether to automatically adjust prices for splits and dividends. (Default: `true`)
    pub fn auto_adjust(mut self, yes: bool) -> Self {
        self.auto_adjust = yes;
        self
    }

    /// Sets whether to include pre-market and post-market data for intraday intervals. (Default: `false`)
    pub fn prepost(mut self, yes: bool) -> Self {
        self.include_prepost = yes;
        self
    }

    /// Sets whether to include corporate actions (dividends and splits) in the response. (Default: `true`)
    pub fn actions(mut self, yes: bool) -> Self {
        self.include_actions = yes;
        self
    }

    /// Sets whether to keep data rows that have missing OHLC values. (Default: `false`)
    ///
    /// If `true`, missing values are represented as `f64::NAN`. If `false`, rows with any missing
    /// OHLC values are dropped.
    pub fn keepna(mut self, yes: bool) -> Self {
        self.keepna = yes;
        self
    }

    /// Executes the request and returns only the price candles.
    pub async fn fetch(self) -> Result<Vec<Candle>, YfError> {
        let resp = self.fetch_full().await?;
        Ok(resp.candles)
    }

    /// Executes the request and returns the full response, including candles, actions, and metadata.
    pub async fn fetch_full(self) -> Result<HistoryResponse, YfError> {
        // 1) Fetch and parse the /chart payload into owned blocks
        let fetched = fetch_chart(
            &self.client,
            &self.symbol,
            self.range,
            self.period,
            self.interval,
            self.include_actions,
            self.include_prepost,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await?;

        // 2) Corporate actions & split ratios
        let (mut actions_out, split_events) = extract_actions(&fetched.events);

        // 3) Cumulative split factors after each bar
        let cum_split_after = cumulative_split_after(&fetched.ts, &split_events);

        // 4) Assemble candles (+ raw close) with/without adjustments
        let (candles, raw_close) = assemble_candles(
            &fetched.ts,
            &fetched.quote,
            &fetched.adjclose,
            self.auto_adjust,
            self.keepna,
            &cum_split_after,
        );

        // ensure actions sorted (extract_actions already sorts, keep consistent)
        actions_out.sort_by_key(|a| match *a {
            Action::Dividend { ts, .. } | Action::Split { ts, .. } => ts,
        });

        // 5) Map metadata
        let meta_out = map_meta(&fetched.meta);

        Ok(HistoryResponse {
            candles,
            actions: actions_out,
            adjusted: self.auto_adjust,
            meta: meta_out,
            raw_close: Some(raw_close),
        })
    }
}

/* --- tiny private helper --- */

fn map_meta(m: &Option<MetaNode>) -> Option<HistoryMeta> {
    m.as_ref().map(|mm| HistoryMeta {
        timezone: mm.timezone.clone(),
        gmtoffset: mm.gmtoffset,
    })
}
