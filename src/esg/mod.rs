mod api;
mod model;
mod wire;

pub use model::{EsgInvolvement, EsgScores};

use crate::{
    YfClient, YfError,
    core::client::{CacheMode, RetryConfig},
};

/// A builder for fetching ESG (Environmental, Social, and Governance) data for a specific symbol.
pub struct EsgBuilder<'a> {
    client: &'a YfClient,
    symbol: String,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl<'a> EsgBuilder<'a> {
    /// Creates a new `EsgBuilder` for a given symbol.
    pub fn new(client: &'a YfClient, symbol: impl Into<String>) -> Self {
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

    /// Fetches the ESG scores and involvement data for the symbol.
    pub async fn fetch(self) -> Result<EsgScores, YfError> {
        api::fetch_esg_scores(
            self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
