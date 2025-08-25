mod model;
mod options;
mod quote;

pub use model::{FastInfo, OptionChain, OptionContract, Quote};

use url::Url;

use crate::{
    YfClient, YfError,
    analysis::AnalysisBuilder,
    core::client::{CacheMode, RetryConfig},
    fundamentals::FundamentalsBuilder,
    history::HistoryBuilder,
};

const DEFAULT_BASE_QUOTE_V7: &str = "https://query1.finance.yahoo.com/v7/finance/quote";
const DEFAULT_BASE_OPTIONS_V7: &str = "https://query1.finance.yahoo.com/v7/finance/options/";

pub struct Ticker {
    pub(crate) client: YfClient,
    pub(crate) symbol: String,
    pub(crate) quote_base: Url,
    options_base: Url,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl Ticker {
    pub fn new(client: YfClient, symbol: impl Into<String>) -> Result<Self, crate::core::YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base: url::Url::parse(DEFAULT_BASE_QUOTE_V7)?,
            options_base: url::Url::parse(DEFAULT_BASE_OPTIONS_V7)?,
            cache_mode: CacheMode::Use,
            retry_override: None,
        })
    }

    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    pub fn with_quote_base(
        client: YfClient,
        symbol: impl Into<String>,
        base: Url,
    ) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base: base,
            options_base: Url::parse(DEFAULT_BASE_OPTIONS_V7)?,
            cache_mode: CacheMode::Use,
            retry_override: None,
        })
    }

    pub fn with_options_base(
        client: YfClient,
        symbol: impl Into<String>,
        base: Url,
    ) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base: Url::parse(DEFAULT_BASE_QUOTE_V7)?,
            options_base: base,
            cache_mode: CacheMode::Use,
            retry_override: None,
        })
    }

    pub fn with_bases(
        client: YfClient,
        symbol: impl Into<String>,
        quote_base: Url,
        options_base: Url,
    ) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base,
            options_base,
            cache_mode: CacheMode::Use,
            retry_override: None,
        })
    }

    pub fn history_builder(&self) -> HistoryBuilder<'_> {
        HistoryBuilder::new(&self.client, &self.symbol)
    }

    /* ---------------- Quotes ---------------- */

    pub async fn quote(&mut self) -> Result<Quote, YfError> {
        quote::fetch_quote(
            &self.client,
            &self.quote_base,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    pub async fn fast_info(&mut self) -> Result<FastInfo, YfError> {
        let q = self.quote().await?;
        let last = q
            .regular_market_price
            .or(q.regular_market_previous_close)
            .ok_or_else(|| YfError::Data("quote missing last/previous price".into()))?;

        Ok(FastInfo {
            symbol: q.symbol,
            last_price: last,
            previous_close: q.regular_market_previous_close,
            currency: q.currency,
            exchange: q.exchange,
            market_state: q.market_state,
        })
    }

    /* ---------------- History helpers ---------------- */

    pub async fn history(
        &self,
        range: Option<crate::Range>,
        interval: Option<crate::Interval>,
        prepost: bool,
    ) -> Result<Vec<crate::Candle>, crate::core::YfError> {
        let mut hb = self.history_builder();
        if let Some(r) = range {
            hb = hb.range(r);
        }
        if let Some(i) = interval {
            hb = hb.interval(i);
        }
        hb = hb
            .auto_adjust(true)
            .prepost(prepost)
            .actions(true)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone());
        hb.fetch().await
    }

    pub async fn actions(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<crate::Action>, YfError> {
        let mut hb = self.history_builder();
        hb = hb.range(range.unwrap_or(crate::Range::Max));
        let resp = hb.auto_adjust(true).actions(true).fetch_full().await?;
        Ok(resp.actions)
    }

    pub async fn dividends(&self, range: Option<crate::Range>) -> Result<Vec<(i64, f64)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Dividend { ts, amount } => Some((ts, amount)),
                _ => None,
            })
            .collect())
    }

    pub async fn splits(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<(i64, u32, u32)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Split {
                    ts,
                    numerator,
                    denominator,
                } => Some((ts, numerator, denominator)),
                _ => None,
            })
            .collect())
    }

    pub async fn get_history_metadata(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Option<crate::HistoryMeta>, crate::core::YfError> {
        let mut hb = self
            .history_builder()
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone());
        if let Some(r) = range {
            hb = hb.range(r);
        }
        let resp = hb.fetch_full().await?;
        Ok(resp.meta)
    }

    /* ---------------- Options ---------------- */

    pub async fn options(&self) -> Result<Vec<i64>, YfError> {
        options::expiration_dates(
            &self.client,
            &self.options_base,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    pub async fn option_chain(&self, date: Option<i64>) -> Result<OptionChain, YfError> {
        options::option_chain(
            &self.client,
            &self.options_base,
            &self.symbol,
            date,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /* ---------------- Analysis convenience ---------------- */

    pub async fn recommendations(&self) -> Result<Vec<crate::RecommendationRow>, YfError> {
        AnalysisBuilder::new(self.client.clone(), &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
            .recommendations()
            .await
    }

    pub async fn recommendations_summary(&self) -> Result<crate::RecommendationSummary, YfError> {
        AnalysisBuilder::new(self.client.clone(), &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
            .recommendations_summary()
            .await
    }

    pub async fn upgrades_downgrades(
        &mut self,
    ) -> Result<Vec<crate::UpgradeDowngradeRow>, YfError> {
        AnalysisBuilder::new(self.client.clone(), &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
            .upgrades_downgrades()
            .await
    }

    pub async fn analyst_price_target(&mut self) -> Result<crate::PriceTarget, YfError> {
        AnalysisBuilder::new(self.client.clone(), &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
            .analyst_price_target()
            .await
    }

    /* ---------------- Fundamentals convenience ---------------- */

    fn fundamentals_builder(&self) -> FundamentalsBuilder {
        FundamentalsBuilder::new(self.client.clone(), &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    pub async fn income_stmt(&mut self) -> Result<Vec<crate::IncomeStatementRow>, YfError> {
        self.fundamentals_builder().income_statement(false).await
    }

    pub async fn quarterly_income_stmt(
        &mut self,
    ) -> Result<Vec<crate::IncomeStatementRow>, YfError> {
        self.fundamentals_builder().income_statement(true).await
    }

    pub async fn balance_sheet(&mut self) -> Result<Vec<crate::BalanceSheetRow>, YfError> {
        self.fundamentals_builder().balance_sheet(false).await
    }

    pub async fn quarterly_balance_sheet(
        &mut self,
    ) -> Result<Vec<crate::BalanceSheetRow>, YfError> {
        self.fundamentals_builder().balance_sheet(true).await
    }

    pub async fn cashflow(&mut self) -> Result<Vec<crate::CashflowRow>, YfError> {
        self.fundamentals_builder().cashflow(false).await
    }

    pub async fn quarterly_cashflow(&mut self) -> Result<Vec<crate::CashflowRow>, YfError> {
        self.fundamentals_builder().cashflow(true).await
    }

    pub async fn earnings(&mut self) -> Result<crate::Earnings, YfError> {
        self.fundamentals_builder().earnings().await
    }

    pub async fn calendar(&mut self) -> Result<crate::FundCalendar, YfError> {
        self.fundamentals_builder().calendar().await
    }
}
