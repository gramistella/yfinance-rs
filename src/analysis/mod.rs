// src/analysis/mod.rs

mod api;
mod model;

mod fetch;
mod wire;

pub use model::{PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};

use crate::{
    YfClient, YfError,
    core::client::{CacheMode, RetryConfig},
};

/// Builder for analysis module API calls.
pub struct AnalysisBuilder {
    client: YfClient,
    symbol: String,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl AnalysisBuilder {
    /// Creates a new builder for a given symbol.
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

    /// Fetches the analyst recommendation trend.
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
    pub async fn recommendations_summary(self) -> Result<RecommendationSummary, YfError> {
        api::recommendation_summary(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the history of analyst upgrades and downgrades.
    pub async fn upgrades_downgrades(self) -> Result<Vec<UpgradeDowngradeRow>, YfError> {
        api::upgrades_downgrades(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the analyst price target.
    pub async fn analyst_price_target(self) -> Result<PriceTarget, YfError> {
        api::analyst_price_target(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
