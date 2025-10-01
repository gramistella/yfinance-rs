mod api;
mod model;

mod fetch;
mod wire;

pub use model::{
    BalanceSheetRow, Calendar, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps,
    EarningsYear, IncomeStatementRow, ShareCount,
};

use crate::core::{
    YfClient, YfError,
    client::{CacheMode, RetryConfig},
};
use paft::money::Currency;

/// A builder for fetching fundamental financial data (statements, earnings, etc.).
pub struct FundamentalsBuilder {
    client: YfClient,
    symbol: String,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl FundamentalsBuilder {
    /// Creates a new `FundamentalsBuilder` for a given symbol.
    pub fn new(client: &YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client: client.clone(),
            symbol: symbol.into(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for this specific API call.
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call.
    #[must_use]
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Fetches the income statement.
    ///
    /// Set `quarterly` to `true` to get quarterly reports, or `false` for annual reports.
    /// Provide `Some(currency)` to override the inferred reporting currency; pass `None`
    /// to use the cached profile-based heuristic.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails or the API response cannot be parsed.
    pub async fn income_statement(
        &self,
        quarterly: bool,
        override_currency: Option<Currency>,
    ) -> Result<Vec<IncomeStatementRow>, YfError> {
        let currency = self
            .client
            .reporting_currency(&self.symbol, override_currency)
            .await;

        api::income_statement(
            &self.client,
            &self.symbol,
            quarterly,
            currency,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the balance sheet.
    ///
    /// Set `quarterly` to `true` to get quarterly reports, or `false` for annual reports.
    /// Provide `Some(currency)` to override the inferred reporting currency; pass `None`
    /// to use the cached profile-based heuristic.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails or the API response cannot be parsed.
    pub async fn balance_sheet(
        &self,
        quarterly: bool,
        override_currency: Option<Currency>,
    ) -> Result<Vec<BalanceSheetRow>, YfError> {
        let currency = self
            .client
            .reporting_currency(&self.symbol, override_currency)
            .await;

        api::balance_sheet(
            &self.client,
            &self.symbol,
            quarterly,
            currency,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the cash flow statement.
    ///
    /// Set `quarterly` to `true` to get quarterly reports, or `false` for annual reports.
    /// Provide `Some(currency)` to override the inferred reporting currency; pass `None`
    /// to use the cached profile-based heuristic.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails or the API response cannot be parsed.
    pub async fn cashflow(
        &self,
        quarterly: bool,
        override_currency: Option<Currency>,
    ) -> Result<Vec<CashflowRow>, YfError> {
        let currency = self
            .client
            .reporting_currency(&self.symbol, override_currency)
            .await;

        api::cashflow(
            &self.client,
            &self.symbol,
            quarterly,
            currency,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches earnings history and estimates.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails or the API response cannot be parsed.
    pub async fn earnings(&self, override_currency: Option<Currency>) -> Result<Earnings, YfError> {
        let currency = self
            .client
            .reporting_currency(&self.symbol, override_currency)
            .await;

        api::earnings(
            &self.client,
            &self.symbol,
            currency,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches corporate calendar events like earnings dates.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails or the API response cannot be parsed.
    pub async fn calendar(&self) -> Result<Calendar, YfError> {
        api::calendar(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the historical number of shares outstanding.
    ///
    /// If `quarterly` is true, fetches quarterly data, otherwise annual data is fetched.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails or the API response cannot be parsed.
    pub async fn shares(&self, quarterly: bool) -> Result<Vec<ShareCount>, YfError> {
        api::shares(
            &self.client,
            &self.symbol,
            None,
            None,
            quarterly,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
