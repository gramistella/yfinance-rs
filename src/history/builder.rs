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

#[derive(Clone)]
pub struct HistoryBuilder<'a> {
    pub(crate) client: &'a YfClient,
    pub(crate) symbol: String,
    pub(crate) range: Option<Range>,
    pub(crate) period: Option<(i64, i64)>,
    pub(crate) interval: Interval,
    pub(crate) auto_adjust: bool,
    pub(crate) include_prepost: bool,
    pub(crate) include_actions: bool,
    pub(crate) keepna: bool,
    pub(crate) cache_mode: CacheMode,
    pub(crate) retry_override: Option<RetryConfig>,
}

impl<'a> HistoryBuilder<'a> {
    pub fn new(client: &'a YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client,
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

    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    pub fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }

    pub fn between(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.range = None;
        self.period = Some((start.timestamp(), end.timestamp()));
        self
    }

    pub fn interval(mut self, interval: Interval) -> Self {
        self.interval = interval;
        self
    }

    pub fn auto_adjust(mut self, yes: bool) -> Self {
        self.auto_adjust = yes;
        self
    }

    pub fn prepost(mut self, yes: bool) -> Self {
        self.include_prepost = yes;
        self
    }

    pub fn actions(mut self, yes: bool) -> Self {
        self.include_actions = yes;
        self
    }

    pub fn keepna(mut self, yes: bool) -> Self {
        self.keepna = yes;
        self
    }

    pub async fn fetch(self) -> Result<Vec<Candle>, YfError> {
        let resp = self.fetch_full().await?;
        Ok(resp.candles)
    }

    pub async fn fetch_full(self) -> Result<HistoryResponse, YfError> {
        // 1) Fetch and parse the /chart payload into owned blocks
        let fetched = fetch_chart(
            self.client,
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
