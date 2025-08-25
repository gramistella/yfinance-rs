use super::wire::{V10Envelope, V10Result};
use crate::core::{YfClient, YfError, net};

#[cfg(any(debug_assertions, feature = "debug-dumps"))]
use crate::profile::debug::debug_dump_api;

/* ---------- Single focused fetch with crumb + retry ---------- */

pub(super) async fn fetch_modules(
    client: &mut YfClient,
    symbol: &str,
    modules: &str,
) -> Result<V10Result, YfError> {
    let env = call_quote_summary(client, symbol, modules).await?;
    get_first_result(env)
}

/* ---------- Internal helpers ---------- */

async fn call_quote_summary(
    client: &mut YfClient,
    symbol: &str,
    modules: &str,
) -> Result<V10Envelope, YfError> {
    for attempt in 0..=1 {
        client.ensure_credentials().await?;

        let crumb = client
            .crumb()
            .ok_or_else(|| YfError::Data("Crumb is not set".into()))?
            .to_string();

        let mut url = client.base_quote_api().join(symbol)?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("modules", modules);
            qp.append_pair("crumb", &crumb);
        }

        if let Some(text) = client.cache_get(&url).await {
            #[cfg(any(debug_assertions, feature = "debug-dumps"))]
            {
                let _ = debug_dump_api(symbol, &text);
            }

            let env: V10Envelope = serde_json::from_str(&text)
                .map_err(|e| YfError::Data(format!("quoteSummary json parse: {e}")))?;

            if let Some(error) = env.quote_summary.as_ref().and_then(|qs| qs.error.as_ref()) {
                let desc = error.description.to_ascii_lowercase();
                if desc.contains("invalid crumb") && attempt == 0 {
                    if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                        eprintln!(
                            "YF_DEBUG: Invalid crumb in analysis (cached); refreshing and retrying."
                        );
                    }
                    client.clear_crumb();
                    continue;
                }
                return Err(YfError::Data(format!("yahoo error: {}", error.description)));
            }

            return Ok(env);
        }

        let resp = client.http().get(url.clone()).send().await?;
        let text = net::get_text(resp, "analysis_api", symbol, "json").await?;

        #[cfg(any(debug_assertions, feature = "debug-dumps"))]
        {
            let _ = debug_dump_api(symbol, &text);
        }

        let env: V10Envelope = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => return Err(YfError::Data(format!("quoteSummary json parse: {e}"))),
        };

        if let Some(error) = env.quote_summary.as_ref().and_then(|qs| qs.error.as_ref()) {
            let desc = error.description.to_ascii_lowercase();
            if desc.contains("invalid crumb") && attempt == 0 {
                if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                    eprintln!("YF_DEBUG: Invalid crumb in analysis; refreshing and retrying.");
                }
                client.clear_crumb();
                continue;
            }
            return Err(YfError::Data(format!("yahoo error: {}", error.description)));
        }

        client.cache_put(&url, &text, None).await;
        return Ok(env);
    }

    Err(YfError::Data("analysis API call failed after retry".into()))
}

fn get_first_result(env: V10Envelope) -> Result<V10Result, YfError> {
    env.quote_summary
        .and_then(|qs| qs.result)
        .and_then(|mut v| v.pop())
        .ok_or_else(|| YfError::Data("empty quoteSummary result".into()))
}
