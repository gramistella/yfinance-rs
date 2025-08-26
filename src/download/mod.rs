use std::collections::HashMap;

use futures::future::try_join_all;

use crate::{
    Action, Candle, HistoryMeta, HistoryResponse, Interval, Range, YfClient, YfError,
    core::client::{CacheMode, RetryConfig},
    history::HistoryBuilder,
};

/// The result of a multi-symbol download operation.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// A map of symbol to its corresponding time series of price `Candle`s.
    pub series: HashMap<String, Vec<Candle>>,
    /// A map of symbol to its historical metadata (timezone, etc.).
    pub meta: HashMap<String, Option<HistoryMeta>>,
    /// A map of symbol to its corporate `Action`s (dividends and splits).
    /// This is only populated if `actions(true)` was set on the builder.
    pub actions: HashMap<String, Vec<Action>>,
    /// `true` if prices were adjusted for splits and dividends.
    pub adjusted: bool,
}

/// A builder for downloading historical data for multiple symbols concurrently.
///
/// This provides a convenient way to fetch data for a list of tickers with the same
/// parameters in parallel, similar to `yfinance.download` in Python.
///
/// Many of the configuration methods mirror those on [`HistoryBuilder`].
pub struct DownloadBuilder<'a> {
    client: &'a YfClient,
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

impl<'a> DownloadBuilder<'a> {
    /// Creates a new `DownloadBuilder`.
    pub fn new(client: &'a YfClient) -> Self {
        Self {
            client,
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
    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for all API calls made by this builder.
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Replaces the current list of symbols with a new list.
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Adds a single symbol to the list of symbols to download.
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }

    /// Sets a relative time range for the request (e.g., `1y`, `6mo`).
    pub fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }

    /// Sets an absolute time period for the request using start and end timestamps.
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

    /// Sets whether to back-adjust prices.
    ///
    /// Back-adjustment adjusts the Open, High, and Low prices, but keeps the Close price as the
    /// raw, unadjusted close. This forces an internal adjustment even if `auto_adjust` is false.
    pub fn back_adjust(mut self, yes: bool) -> Self {
        self.back_adjust = yes;
        self
    }

    /// Sets whether to include pre-market and post-market data for intraday intervals. (Default: `false`)
    pub fn prepost(mut self, yes: bool) -> Self {
        self.include_prepost = yes;
        self
    }

    /// Sets whether to include corporate actions (dividends and splits) in the result. (Default: `true`)
    pub fn actions(mut self, yes: bool) -> Self {
        self.include_actions = yes;
        self
    }

    /// Sets whether to keep data rows that have missing OHLC values. (Default: `false`)
    pub fn keepna(mut self, yes: bool) -> Self {
        self.keepna = yes;
        self
    }

    /// Sets whether to round prices to 2 decimal places. (Default: `false`)
    pub fn rounding(mut self, yes: bool) -> Self {
        self.rounding = yes;
        self
    }

    /// Sets whether to attempt to repair obvious price outliers (e.g., 100x errors). (Default: `false`)
    pub fn repair(mut self, yes: bool) -> Self {
        self.repair = yes;
        self
    }

    /// Executes the download by fetching data for all specified symbols concurrently.
    pub async fn run(self) -> Result<DownloadResult, YfError> {
        if self.symbols.is_empty() {
            return Err(YfError::Data("no symbols specified".into()));
        }

        let need_adjust_in_fetch = self.auto_adjust || self.back_adjust;

        // Precompute period timestamps here so we can use `?` safely
        let period_dt = if let Some((p1, p2)) = self.period {
            use chrono::{TimeZone, Utc};
            let start = Utc
                .timestamp_opt(p1, 0)
                .single()
                .ok_or_else(|| YfError::Data("invalid period1".into()))?;
            let end = Utc
                .timestamp_opt(p2, 0)
                .single()
                .ok_or_else(|| YfError::Data("invalid period2".into()))?;
            Some((start, end))
        } else {
            None
        };

        let futures = self.symbols.iter().map(|sym| {
            let sym = sym.clone();
            let mut hb: HistoryBuilder<'_> = HistoryBuilder::new(self.client, sym.clone())
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

            async move {
                let full: HistoryResponse = hb.fetch_full().await?;
                Ok::<(String, HistoryResponse), YfError>((sym, full))
            }
        });

        let joined: Vec<(String, HistoryResponse)> = try_join_all(futures).await?;

        let mut series: std::collections::HashMap<String, Vec<Candle>> =
            std::collections::HashMap::new();
        let mut meta: std::collections::HashMap<String, Option<HistoryMeta>> =
            std::collections::HashMap::new();
        let mut actions: std::collections::HashMap<String, Vec<Action>> =
            std::collections::HashMap::new();

        for (sym, mut resp) in joined {
            let mut v = resp.candles;

            // Keep your current "back_adjust" semantics but avoid non-finite writes
            if self.back_adjust
                && let Some(raw) = resp.raw_close.take()
            {
                for (i, c) in v.iter_mut().enumerate() {
                    if let Some(&rc) = raw.get(i)
                        && rc.is_finite()
                    {
                        c.close = rc;
                    }
                }
            }

            if self.repair {
                repair_scale_outliers(&mut v);
            }

            if self.rounding {
                for c in &mut v {
                    if c.open.is_finite() {
                        c.open = round2(c.open);
                    }
                    if c.high.is_finite() {
                        c.high = round2(c.high);
                    }
                    if c.low.is_finite() {
                        c.low = round2(c.low);
                    }
                    if c.close.is_finite() {
                        c.close = round2(c.close);
                    }
                }
            }

            if self.include_actions {
                actions.insert(sym.clone(), resp.actions);
            }
            meta.insert(sym.clone(), resp.meta);
            series.insert(sym, v);
        }

        Ok(DownloadResult {
            series,
            meta,
            actions,
            adjusted: need_adjust_in_fetch,
        })
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

        let p = prev.close;
        let n = next.close;
        let c = cur.close;

        if !(p.is_finite() && n.is_finite() && c.is_finite()) {
            continue;
        }

        let baseline = (p + n) / 2.0;
        if baseline <= 0.0 {
            continue;
        }

        let ratio = c / baseline;

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
            continue;
        }
    }
}

fn scale_row_prices(c: &mut Candle, scale: f64) {
    if c.open.is_finite() {
        c.open *= scale;
    }
    if c.high.is_finite() {
        c.high *= scale;
    }
    if c.low.is_finite() {
        c.low *= scale;
    }
    if c.close.is_finite() {
        c.close *= scale;
    }
}
