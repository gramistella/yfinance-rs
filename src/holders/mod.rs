mod api;
mod model;
mod wire;

pub use model::{
    InsiderRosterHolder, InsiderTransaction, InstitutionalHolder, MajorHolder,
    NetSharePurchaseActivity,
};

use crate::{
    YfClient, YfError,
    core::client::{CacheMode, RetryConfig},
};

/// A builder for fetching holder data for a specific symbol.
pub struct HoldersBuilder {
    client: YfClient,
    symbol: String,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl HoldersBuilder {
    /// Creates a new `HoldersBuilder` for a given symbol.
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

    /// Fetches the major holders breakdown (e.g., % insiders, % institutions).
    pub async fn major_holders(&self) -> Result<Vec<MajorHolder>, YfError> {
        api::major_holders(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches a list of the top institutional holders.
    pub async fn institutional_holders(&self) -> Result<Vec<InstitutionalHolder>, YfError> {
        api::institutional_holders(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches a list of the top mutual fund holders.
    pub async fn mutual_fund_holders(&self) -> Result<Vec<InstitutionalHolder>, YfError> {
        api::mutual_fund_holders(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches a list of recent insider transactions.
    pub async fn insider_transactions(&self) -> Result<Vec<InsiderTransaction>, YfError> {
        api::insider_transactions(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches a roster of company insiders and their holdings.
    pub async fn insider_roster_holders(&self) -> Result<Vec<InsiderRosterHolder>, YfError> {
        api::insider_roster_holders(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches a summary of net insider purchase and sale activity.
    pub async fn net_share_purchase_activity(
        &self,
    ) -> Result<Option<NetSharePurchaseActivity>, YfError> {
        api::net_share_purchase_activity(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
