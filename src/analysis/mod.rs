mod api;
mod model;

/* new: split internals */
mod fetch;
mod wire;

pub use model::{PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};

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

pub async fn analyst_price_target(
    client: &mut YfClient,
    symbol: &str,
) -> Result<PriceTarget, YfError> {
    api::analyst_price_target(client, symbol).await
}
