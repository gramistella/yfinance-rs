mod api;
mod model;

mod fetch;
mod wire;

pub use model::{
    EarningsTrendRow, PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow,
};

use crate::{
    YfClient, YfError,
    core::client::{CacheMode, RetryConfig},
};

/// A builder for fetching analyst-related data for a specific symbol.
pub struct AnalysisBuilder {
    client: YfClient,
    symbol: String,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl AnalysisBuilder {
    /// Creates a new `AnalysisBuilder` for a given symbol.
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

    /// Fetches the analyst recommendation trend over time.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the data is malformed.
    pub async fn recommendations(self) -> Result<Vec<RecommendationRow>, YfError> {
        api::recommendation_trend(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches a summary of the latest analyst recommendations.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the data is malformed.
    pub async fn recommendations_summary(self) -> Result<RecommendationSummary, YfError> {
        api::recommendation_summary(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the history of analyst upgrades and downgrades for the symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the data is malformed.
    pub async fn upgrades_downgrades(self) -> Result<Vec<UpgradeDowngradeRow>, YfError> {
        api::upgrades_downgrades(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the analyst price target summary.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the data is malformed.
    pub async fn analyst_price_target(self) -> Result<PriceTarget, YfError> {
        api::analyst_price_target(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches earnings trend data.
    ///
    /// This includes earnings estimates, revenue estimates, EPS trends, and EPS revisions.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the data is malformed.
    pub async fn earnings_trend(self) -> Result<Vec<EarningsTrendRow>, YfError> {
        api::earnings_trend(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
