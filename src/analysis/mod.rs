mod api;
mod model;

pub use model::{RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};

use crate::{YfClient, YfError};

pub async fn recommendations(
    client: &mut YfClient,
    symbol: &str,
) -> Result<Vec<RecommendationRow>, YfError> {
    api::recommendation_trend(client, symbol).await
}

pub async fn recommendations_summary(
    client: &mut YfClient,
    symbol: &str,
) -> Result<RecommendationSummary, YfError> {
    api::recommendation_summary(client, symbol).await
}

pub async fn upgrades_downgrades(
    client: &mut YfClient,
    symbol: &str,
) -> Result<Vec<UpgradeDowngradeRow>, YfError> {
    api::upgrades_downgrades(client, symbol).await
}
