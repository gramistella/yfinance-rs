use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use futures::future::try_join_all;

use crate::{
    history::{HistoryBuilder, HistoryMeta, HistoryResponse, Interval, Range},
    Action, Candle, YfClient, YfError,
};

/// Result of a multi-symbol download.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub series: HashMap<String, Vec<Candle>>,
    pub meta: HashMap<String, Option<HistoryMeta>>,
    pub actions: HashMap<String, Vec<Action>>,
    pub adjusted: bool,
}

pub struct DownloadBuilder<'a> {
    client: &'a YfClient,
    symbols: Vec<String>,
    range: Option<Range>,
    period: Option<(i64, i64)>,
    interval: Interval,
    auto_adjust: bool,
    include_prepost: bool,
    include_actions: bool,
    _back_adjust: bool,
    _repair: bool,
    _keepna: bool,
    _rounding: Option<u32>,
}

impl<'a> DownloadBuilder<'a> {
    pub fn new(client: &'a YfClient) -> Self {
        Self {
            client,
            symbols: Vec::new(),
            range: Some(Range::M6),
            period: None,
            interval: Interval::D1,
            auto_adjust: true,
            include_prepost: false,
            include_actions: true,
            _back_adjust: false,
            _repair: false,
            _keepna: false,
            _rounding: None,
        }
    }

    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(|s| s.into()).collect();
        self
    }
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }
    pub fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }
    pub fn between(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
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
    pub fn back_adjust(mut self, yes: bool) -> Self {
        self._back_adjust = yes;
        self
    }
    pub fn repair(mut self, yes: bool) -> Self {
        self._repair = yes;
        self
    }
    pub fn keepna(mut self, yes: bool) -> Self {
        self._keepna = yes;
        self
    }
    pub fn rounding(mut self, digits: Option<u32>) -> Self {
        self._rounding = digits;
        self
    }

    pub async fn run(self) -> Result<DownloadResult, YfError> {
        if self.symbols.is_empty() {
            return Err(YfError::Data("no symbols specified".into()));
        }

        let futures = self.symbols.iter().map(|sym| {
            let sym = sym.clone();
            let mut hb = HistoryBuilder::new(self.client, sym.clone())
                .interval(self.interval)
                .auto_adjust(self.auto_adjust)
                .prepost(self.include_prepost)
                .actions(self.include_actions);

            if let Some((p1, p2)) = self.period {
                let start = Utc.timestamp_opt(p1, 0).single()
                    .ok_or(YfError::Data("invalid period1".into()))
                    .unwrap();
                let end = Utc.timestamp_opt(p2, 0).single()
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

        let mut series = HashMap::new();
        let mut meta = HashMap::new();
        let mut actions = HashMap::new();

        for (sym, resp) in joined {
            series.insert(sym.clone(), resp.candles);
            meta.insert(sym.clone(), resp.meta);
            if self.include_actions {
                actions.insert(sym, resp.actions);
            }
        }

        Ok(DownloadResult {
            series,
            meta,
            actions,
            adjusted: self.auto_adjust,
        })
    }
}
