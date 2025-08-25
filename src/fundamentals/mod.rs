mod api;
mod model;

mod fetch;
mod wire;

pub use model::{
    BalanceSheetRow, Calendar, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps,
    EarningsYear, IncomeStatementRow, Num,
};

use crate::{
    YfClient, YfError,
    core::client::{CacheMode, RetryConfig},
};

pub struct FundamentalsBuilder {
    client: YfClient,
    symbol: String,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl FundamentalsBuilder {
    pub fn new(client: YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client,
            symbol: symbol.into(),
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

    pub async fn income_statement(
        self,
        quarterly: bool,
    ) -> Result<Vec<IncomeStatementRow>, YfError> {
        api::income_statement(
            &self.client,
            &self.symbol,
            quarterly,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    pub async fn balance_sheet(self, quarterly: bool) -> Result<Vec<BalanceSheetRow>, YfError> {
        api::balance_sheet(
            &self.client,
            &self.symbol,
            quarterly,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    pub async fn cashflow(self, quarterly: bool) -> Result<Vec<CashflowRow>, YfError> {
        api::cashflow(
            &self.client,
            &self.symbol,
            quarterly,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    pub async fn earnings(self) -> Result<Earnings, YfError> {
        api::earnings(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    pub async fn calendar(self) -> Result<Calendar, YfError> {
        api::calendar(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
