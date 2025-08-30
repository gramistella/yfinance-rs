use crate::core::{
    YfClient, YfError,
    client::{CacheMode, RetryConfig},
    net,
};
use serde::Deserialize;

#[cfg(feature = "debug-dumps")]
use crate::profile::debug::debug_dump_api;

#[derive(Deserialize)]
pub struct V10Envelope {
    #[serde(rename = "quoteSummary")]
    pub(crate) quote_summary: Option<V10QuoteSummary>,
}

#[derive(Deserialize)]
pub struct V10QuoteSummary {
    pub(crate) result: Option<Vec<serde_json::Value>>,
    pub(crate) error: Option<V10Error>,
}

#[derive(Deserialize)]
pub struct V10Error {
    pub(crate) description: String,
}

pub async fn fetch(
    client: &YfClient,
    symbol: &str,
    modules: &str,
    caller: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<V10Envelope, YfError> {
    async fn attempt_fetch(
        client: &YfClient,
        symbol: &str,
        modules: &str,
        caller: &str,
        cache_mode: CacheMode,
        retry_override: Option<&RetryConfig>,
    ) -> Result<V10Envelope, YfError> {
        client.ensure_credentials().await?;

        let crumb = client
            .crumb()
            .await
            .ok_or_else(|| YfError::Data("Crumb is not set".into()))?;

        let mut url = client.base_quote_api().join(symbol)?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("modules", modules);
            qp.append_pair("crumb", &crumb);
        }

        if cache_mode == CacheMode::Use
            && let Some(text) = client.cache_get(&url).await
        {
            #[cfg(feature = "debug-dumps")]
            let _ = debug_dump_api(symbol, &text);
            return serde_json::from_str(&text)
                .map_err(|e| YfError::Data(format!("quoteSummary json parse (cache): {e}")));
        }

        let req = client.http().get(url.clone());
        let resp = client.send_with_retry(req, retry_override).await?;

        // Create a sanitized key from module names for a unique fixture filename.
        let module_key = modules
            .replace(',', "-")
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
        let fixture_endpoint = format!("{caller}_api_{module_key}");
        let text = net::get_text(resp, &fixture_endpoint, symbol, "json").await?;

        #[cfg(feature = "debug-dumps")]
        let _ = debug_dump_api(symbol, &text);

        if cache_mode != CacheMode::Bypass {
            client.cache_put(&url, &text, None).await;
        }

        serde_json::from_str(&text)
            .map_err(|e| YfError::Data(format!("quoteSummary json parse: {e}")))
    }

    for attempt in 0..=1 {
        let env =
            attempt_fetch(client, symbol, modules, caller, cache_mode, retry_override).await?;

        if let Some(error) = env.quote_summary.as_ref().and_then(|qs| qs.error.as_ref()) {
            let desc = error.description.to_ascii_lowercase();
            if desc.contains("invalid crumb") && attempt == 0 {
                if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                    eprintln!("YF_DEBUG: Invalid crumb in {caller}; refreshing and retrying.");
                }
                client.clear_crumb().await;
                continue;
            }
            return Err(YfError::Data(format!("yahoo error: {}", error.description)));
        }

        return Ok(env);
    }

    Err(YfError::Data(format!(
        "{caller} API call failed after retry"
    )))
}

pub async fn fetch_module_result<T>(
    client: &YfClient,
    symbol: &str,
    modules: &str,
    caller: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<T, YfError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let env = fetch(client, symbol, modules, caller, cache_mode, retry_override).await?;

    let result_val = env
        .quote_summary
        .and_then(|qs| qs.result)
        .and_then(|mut v| v.pop())
        .ok_or_else(|| YfError::Data("empty quoteSummary result".into()))?;

    serde_json::from_value(result_val)
        .map_err(|e| YfError::Data(format!("quoteSummary result parse: {e}")))
}
