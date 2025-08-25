use super::wire::V10Result;
use crate::core::{
    YfClient, YfError,
    client::{CacheMode, RetryConfig},
    quotesummary,
};

pub(super) async fn fetch_modules(
    client: &mut YfClient,
    symbol: &str,
    modules: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<V10Result, YfError> {
    quotesummary::fetch_module_result(
        client,
        symbol,
        modules,
        "analysis",
        cache_mode,
        retry_override,
    )
    .await
}
