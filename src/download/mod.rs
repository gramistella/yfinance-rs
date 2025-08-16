use std::collections::HashMap;

use futures::future::try_join_all;

use crate::{
    Action, Candle, HistoryMeta, HistoryResponse, Interval, Range, YfClient, YfError,
    history::HistoryBuilder,
};

/// Result of a multi-symbol download.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// Adjusted (per builder settings) OHLCV series per symbol.
    pub series: HashMap<String, Vec<Candle>>,
    /// History metadata (timezone/gmtoffset) per symbol.
    pub meta: HashMap<String, Option<HistoryMeta>>,
    /// Corporate actions per symbol (only populated if `actions(true)` on the builder).
    pub actions: HashMap<String, Vec<Action>>,
    /// Whether prices were adjusted (true if `auto_adjust` OR `back_adjust` were applied).
    pub adjusted: bool,
}

/// Multi-symbol history downloader similar to `yfinance.download`.
///
/// Parity knobs supported:
/// - `auto_adjust(true)`: adjust OHLC (incl. Close) using adjclose/splits
/// - `back_adjust(true)`: adjust O/H/L but keep Close as the *raw* close
/// - `keepna(true)`: keep rows with missing OHLC (filled with NaN)
/// - `rounding(true)`: round prices to 2 decimals
/// - `repair(true)`: fix obvious 100× outliers in price rows (simple heuristic)
///
/// Notes:
/// - If `back_adjust(true)` is used, the internal fetch will force adjustment so
///   we can back-fill O/H/L while preserving raw Close values.
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
}

impl<'a> DownloadBuilder<'a> {
    /// Start a new multi-symbol download builder.
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
        }
    }

    /// Replace the full symbol list.
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Add a single symbol.
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }

    /// Use a relative range (e.g., 1d, 6mo, ytd, 10y, max).
    pub fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }

    /// Use absolute start/end timestamps instead of a range.
    pub fn between(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.range = None;
        self.period = Some((start.timestamp(), end.timestamp()));
        self
    }

    /// Set the bar interval (default: 1d).
    pub fn interval(mut self, interval: Interval) -> Self {
        self.interval = interval;
        self
    }

    /// Auto-adjust OHLC using adjclose and split factors (default: true).
    pub fn auto_adjust(mut self, yes: bool) -> Self {
        self.auto_adjust = yes;
        self
    }

    /// Back-adjust: adjust O/H/L but keep Close as the *raw* (unadjusted) close.
    /// This will force internal adjustment of O/H/L even if `auto_adjust(false)`.
    pub fn back_adjust(mut self, yes: bool) -> Self {
        self.back_adjust = yes;
        self
    }

    /// Include pre/post-market bars for intraday (default: false).
    pub fn prepost(mut self, yes: bool) -> Self {
        self.include_prepost = yes;
        self
    }

    /// Include dividends/splits in output `actions` (default: true).
    pub fn actions(mut self, yes: bool) -> Self {
        self.include_actions = yes;
        self
    }

    /// Keep rows with missing OHLC (as NaN). Default false (drop NA rows).
    pub fn keepna(mut self, yes: bool) -> Self {
        self.keepna = yes;
        self
    }

    /// Round prices to 2 decimals (yfinance default when rounding enabled).
    pub fn rounding(mut self, yes: bool) -> Self {
        self.rounding = yes;
        self
    }

    /// Repair obvious 100× spikes/dips (currency/outlier fix).
    pub fn repair(mut self, yes: bool) -> Self {
        self.repair = yes;
        self
    }

    /// Execute the download concurrently and collect results.
    ///
    /// Fails fast if any symbol fetch fails.
    pub async fn run(self) -> Result<DownloadResult, YfError> {
        if self.symbols.is_empty() {
            return Err(YfError::Data("no symbols specified".into()));
        }

        let need_adjust_in_fetch = self.auto_adjust || self.back_adjust;

        // Build a future per symbol (polled concurrently).
        let futures = self.symbols.iter().map(|sym| {
            let sym = sym.clone();
            let mut hb = HistoryBuilder::new(self.client, sym.clone())
                .interval(self.interval)
                .auto_adjust(need_adjust_in_fetch)
                .prepost(self.include_prepost)
                .actions(self.include_actions)
                .keepna(self.keepna);

            if let Some((p1, p2)) = self.period {
                use chrono::{TimeZone, Utc};
                let start = Utc
                    .timestamp_opt(p1, 0)
                    .single()
                    .ok_or(YfError::Data("invalid period1".into()))
                    .unwrap();
                let end = Utc
                    .timestamp_opt(p2, 0)
                    .single()
                    .ok_or(YfError::Data("invalid period2".into()))
                    .unwrap();
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

        let mut series: HashMap<String, Vec<Candle>> = HashMap::new();
        let mut meta: HashMap<String, Option<HistoryMeta>> = HashMap::new();
        let mut actions: HashMap<String, Vec<Action>> = HashMap::new();

        for (sym, mut resp) in joined {
            let mut v = resp.candles;

            // back_adjust: override Close with raw (unadjusted) close values.
            if self.back_adjust
                && let Some(raw) = resp.raw_close.take()
            {
                for (i, c) in v.iter_mut().enumerate() {
                    if let Some(&rc) = raw.get(i) {
                        if rc.is_finite() {
                            c.close = rc;
                        } else {
                            // keep as-is (NaN stays if keepna)
                            c.close = rc;
                        }
                    }
                }
            }

            // repair: fix 100× outliers in place
            if self.repair {
                repair_scale_outliers(&mut v);
            }

            // rounding: round prices to 2 dp
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
