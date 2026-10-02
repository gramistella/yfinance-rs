use serde::Deserialize;

use crate::core::{CallOptions, YfClient, YfError, quotesummary};

#[derive(Deserialize)]
struct ComponentsResult {
    components: Option<ComponentsNode>,
}

#[derive(Deserialize)]
struct ComponentsNode {
    #[serde(default)]
    components: Vec<String>,
}

pub async fn fetch_components(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<Vec<String>, YfError> {
    let result = quotesummary::fetch_module_result::<ComponentsResult>(
        client,
        symbol,
        "components",
        "components",
        options,
    )
    .await;

    // Yahoo answers 404 for symbols without components (equities, ETFs, ...).
    let root = match result {
        Ok(root) => root,
        Err(YfError::NotFound { .. }) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    Ok(root
        .components
        .map(|node| node.components)
        .unwrap_or_default())
}
