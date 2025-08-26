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

/// A builder for fetching fundamental financial data (statements, earnings, etc.).
pub struct FundamentalsBuilder {
    client: YfClient,
    symbol: String,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl FundamentalsBuilder {
    /// Creates a new `FundamentalsBuilder` for a given symbol.
    pub fn new(client: YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client,
            symbol: symbol.into(),
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

    /// Fetches the income statement.
    ///
    /// Set `quarterly` to `true` to get quarterly reports, or `false` for annual reports.
    pub async fn income_statement(
        &self,
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

    /// Fetches the balance sheet.
    ///
    /// Set `quarterly` to `true` to get quarterly reports, or `false` for annual reports.
    pub async fn balance_sheet(&self, quarterly: bool) -> Result<Vec<BalanceSheetRow>, YfError> {
        api::balance_sheet(
            &self.client,
            &self.symbol,
            quarterly,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the cash flow statement.
    ///
    /// Set `quarterly` to `true` to get quarterly reports, or `false` for annual reports.
    pub async fn cashflow(&self, quarterly: bool) -> Result<Vec<CashflowRow>, YfError> {
        api::cashflow(
            &self.client,
            &self.symbol,
            quarterly,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches earnings history and estimates.
    pub async fn earnings(&self) -> Result<Earnings, YfError> {
        api::earnings(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches corporate calendar events like earnings dates.
    pub async fn calendar(&self) -> Result<Calendar, YfError> {
        api::calendar(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
