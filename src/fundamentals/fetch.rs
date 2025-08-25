use super::wire::V10Result;
use crate::core::{YfClient, YfError, client::CacheMode, net, quotesummary};

/* ---------- Single focused fetch with crumb + retry ---------- */

pub(super) async fn fetch_modules(
    client: &mut YfClient,
    symbol: &str,
    modules: &str,
) -> Result<V10Result, YfError> {
    quotesummary::fetch_module_result(
        client,
        symbol,
        modules,
        "fundamentals",
        CacheMode::Use,
        None,
    )
    .await
}
