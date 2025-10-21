use futures::future::try_join_all;

use crate::{
    core::client::{CacheMode, RetryConfig},
    core::{Candle, HistoryResponse, Interval, Range, YfClient, YfError},
    history::HistoryBuilder,
};
use paft::domain::{AssetKind, Instrument};
use paft::market::responses::download::{DownloadEntry, DownloadResponse};
use paft::money::Money;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
type DateRange = (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>);
type MaybeDateRange = Option<DateRange>;

/// A builder for downloading historical data for multiple symbols concurrently.
///
/// This provides a convenient way to fetch data for a list of tickers with the same
/// parameters in parallel, similar to `yfinance.download` in Python.
///
/// Many of the configuration methods mirror those on [`HistoryBuilder`].
#[allow(clippy::struct_excessive_bools)]
pub struct DownloadBuilder {
    client: YfClient,
    symbols: Vec<String>,

    // date / time controls
    range: Option<Range>,
    period: Option<(i64, i64)>,
    interval: Interval,

    // behavior flags
    auto_adjust: bool,
    back_adjust: bool,
    include_prepost: bool,
    include_actions: bool,
    keepna: bool,
    rounding: bool,
    repair: bool,

    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
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
            .keepna(self.keepna)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone());

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
        if !self.back_adjust {
            return;
        }
        for c in rows.iter_mut() {
            if let Some(rc) = c.close_unadj.as_ref()
                && rc.amount().to_f64().is_some_and(f64::is_finite)
            {
                c.close = rc.clone();
            }
        }
    }

    fn apply_rounding_if_enabled(&self, rows: &mut [Candle]) {
        if !self.rounding {
            return;
        }
        for c in rows {
            if c.open.amount().to_f64().is_some_and(f64::is_finite) {
                c.open = Money::new(
                    rust_decimal::Decimal::from_f64(round2(
                        c.open.amount().to_f64().unwrap_or(0.0),
                    ))
                    .unwrap_or_default(),
                    c.open.currency().clone(),
                )
                .expect("currency metadata available");
            }
            if c.high.amount().to_f64().is_some_and(f64::is_finite) {
                c.high = Money::new(
                    rust_decimal::Decimal::from_f64(round2(
                        c.high.amount().to_f64().unwrap_or(0.0),
                    ))
                    .unwrap_or_default(),
                    c.high.currency().clone(),
                )
                .expect("currency metadata available");
            }
            if c.low.amount().to_f64().is_some_and(f64::is_finite) {
                c.low = Money::new(
                    rust_decimal::Decimal::from_f64(round2(c.low.amount().to_f64().unwrap_or(0.0)))
                        .unwrap_or_default(),
                    c.low.currency().clone(),
                )
                .expect("currency metadata available");
            }
            if c.close.amount().to_f64().is_some_and(f64::is_finite) {
                c.close = Money::new(
                    rust_decimal::Decimal::from_f64(round2(
                        c.close.amount().to_f64().unwrap_or(0.0),
                    ))
                    .unwrap_or_default(),
                    c.close.currency().clone(),
                )
                .expect("currency metadata available");
            }
        }
    }

    fn maybe_repair(&self, rows: &mut [Candle]) {
        if self.repair {
            repair_scale_outliers(rows);
        }
    }

    async fn process_joined_results(
        &self,
        joined: Vec<(String, HistoryResponse)>,
        _need_adjust_in_fetch: bool,
    ) -> DownloadResponse {
        let mut entries: Vec<DownloadEntry> = Vec::with_capacity(joined.len());
        for (sym, mut resp) in joined {
            // apply transforms to candles
            self.apply_back_adjust(&mut resp.candles);
            self.maybe_repair(&mut resp.candles);
            self.apply_rounding_if_enabled(&mut resp.candles);

            // get instrument from cache or fallback
            let instrument = if let Some(inst) = self.client.cached_instrument(&sym).await {
                inst
            } else {
                let kind = AssetKind::Equity;
                let inst = Instrument::from_symbol(&sym, kind).expect("valid symbol");
                self.client
                    .store_instrument(sym.clone(), inst.clone())
                    .await;
                inst
            };

            entries.push(DownloadEntry {
                instrument,
                history: resp,
            });
        }
        DownloadResponse { entries }
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
            auto_adjust: true,
            back_adjust: false,
            include_prepost: false,
            include_actions: true,
            keepna: false,
            rounding: false,
            repair: false,
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for all API calls made by this builder.
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for all API calls made by this builder.
    #[must_use]
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
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

    /// Sets whether to automatically adjust prices for splits and dividends. (Default: `true`)
    #[must_use]
    pub const fn auto_adjust(mut self, yes: bool) -> Self {
        self.auto_adjust = yes;
        self
    }

    /// Sets whether to back-adjust prices.
    ///
    /// Back-adjustment adjusts the Open, High, and Low prices, but keeps the Close price as the
    /// raw, unadjusted close. This forces an internal adjustment even if `auto_adjust` is false.
    #[must_use]
    pub const fn back_adjust(mut self, yes: bool) -> Self {
        self.back_adjust = yes;
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

    /// Sets whether to keep data rows that have missing OHLC values. (Default: `false`)
    #[must_use]
    pub const fn keepna(mut self, yes: bool) -> Self {
        self.keepna = yes;
        self
    }

    /// Sets whether to round prices to 2 decimal places. (Default: `false`)
    #[must_use]
    pub const fn rounding(mut self, yes: bool) -> Self {
        self.rounding = yes;
        self
    }

    /// Sets whether to attempt to repair obvious price outliers (e.g., 100x errors). (Default: `false`)
    #[must_use]
    pub const fn repair(mut self, yes: bool) -> Self {
        self.repair = yes;
        self
    }

    /// Executes the download by fetching data for all specified symbols concurrently.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the underlying history requests fail.
    pub async fn run(self) -> Result<DownloadResponse, YfError> {
        if self.symbols.is_empty() {
            return Err(YfError::InvalidParams("no symbols specified".into()));
        }

        let need_adjust_in_fetch = self.auto_adjust || self.back_adjust;
        let period_dt = self.precompute_period_dt()?;

        let futures = self.symbols.iter().map(|sym| {
            let sym = sym.clone();
            let hb = self.build_history_for_symbol(&sym, period_dt, need_adjust_in_fetch);

            async move {
                let full: HistoryResponse = hb.fetch_full().await?;
                Ok::<(String, HistoryResponse), YfError>((sym, full))
            }
        });

        let joined: Vec<(String, HistoryResponse)> = try_join_all(futures).await?;
        Ok(self
            .process_joined_results(joined, need_adjust_in_fetch)
            .await)
    }
}

/* ---------------- internal helpers ---------------- */

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Very lightweight "repair" pass:
/// If a bar's close is ~100× the average of its neighbors (or ~1/100),
/// scale that entire bar's OHLC accordingly. Volumes are left unchanged.
fn repair_scale_outliers(rows: &mut [Candle]) {
    if rows.len() < 3 {
        return;
    }

    for i in 1..rows.len() - 1 {
        // Split rows at i, so left[..i] and right[i..] don't overlap.
        let (left, right) = rows.split_at_mut(i);

        // prev is in the left side (immutable is fine)
        let prev = &left[i - 1];

        // Now split the right side so we can mutably borrow the “current” bar
        // and (immutably) the remainder where “next” lives, without overlap.
        let (cur, rem) = right.split_first_mut().expect("right has at least 1");
        let next = &rem[0]; // safe because len >= 2 overall ⇒ rem has at least one

        let p = &prev.close;
        let n = &next.close;
        let c = &cur.close;

        if !(p.amount().to_f64().is_some_and(f64::is_finite)
            && n.amount().to_f64().is_some_and(f64::is_finite)
            && c.amount().to_f64().is_some_and(f64::is_finite))
        {
            continue;
        }

        let p_val = p.amount().to_f64().unwrap_or(0.0);
        let n_val = n.amount().to_f64().unwrap_or(0.0);
        let c_val = c.amount().to_f64().unwrap_or(0.0);

        let baseline = f64::midpoint(p_val, n_val);
        if baseline <= 0.0 {
            continue;
        }

        let ratio = c_val / baseline;

        // ~100× high
        if ratio > 50.0 && ratio < 200.0 {
            let scale = if (80.0..125.0).contains(&ratio) {
                0.01
            } else {
                1.0 / ratio
            };
            scale_row_prices(cur, scale);
            continue;
        }

        // ~100× low
        if ratio > 0.0 && ratio < 0.02 {
            let scale = if (0.008..0.0125).contains(&ratio) {
                100.0
            } else {
                1.0 / ratio
            };
            scale_row_prices(cur, scale);
        }
    }
}

fn scale_row_prices(c: &mut Candle, scale: f64) {
    if c.open.amount().to_f64().is_some_and(f64::is_finite) {
        c.open = c
            .open
            .try_mul(rust_decimal::Decimal::from_f64_retain(scale).unwrap_or_default())
            .expect("currency metadata available");
    }
    if c.high.amount().to_f64().is_some_and(f64::is_finite) {
        c.high = c
            .high
            .try_mul(rust_decimal::Decimal::from_f64_retain(scale).unwrap_or_default())
            .expect("currency metadata available");
    }
    if c.low.amount().to_f64().is_some_and(f64::is_finite) {
        c.low = c
            .low
            .try_mul(rust_decimal::Decimal::from_f64_retain(scale).unwrap_or_default())
            .expect("currency metadata available");
    }
    if c.close.amount().to_f64().is_some_and(f64::is_finite) {
        c.close = c
            .close
            .try_mul(rust_decimal::Decimal::from_f64_retain(scale).unwrap_or_default())
            .expect("currency metadata available");
    }
}
