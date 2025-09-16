use serde::Serialize;

use crate::{
    core::{
        YfClient, YfError,
        client::{CacheMode, RetryConfig},
        conversions::*,
        net,
    },
    news::{NewsTab, model::NewsArticle, tab_as_str, wire},
};

#[derive(Serialize)]
struct ServiceConfig<'a> {
    #[serde(rename = "snippetCount")]
    snippet_count: u32,
    s: &'a [&'a str],
}

#[derive(Serialize)]
struct NewsPayload<'a> {
    #[serde(rename = "serviceConfig")]
    service_config: ServiceConfig<'a>,
}

pub(super) async fn fetch_news(
    client: &YfClient,
    symbol: &str,
    count: u32,
    tab: NewsTab,
    _cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<NewsArticle>, YfError> {
    let mut url = client.base_news().join("xhr/ncp")?;
    url.query_pairs_mut()
        .append_pair("queryRef", tab_as_str(tab))
        .append_pair("serviceKey", "ncp_fin");

    let payload = NewsPayload {
        service_config: ServiceConfig {
            snippet_count: count,
            s: &[symbol],
        },
    };

    // Note: The client's built-in cache is URL-based and doesn't support POST bodies.
    // Caching for this endpoint would require a more complex keying strategy.

    let req = client.http().post(url).json(&payload);
    let resp = client.send_with_retry(req, retry_override).await?;

    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        let url_s = resp.url().to_string();
        return Err(match code {
            404 => YfError::NotFound { url: url_s },
            429 => YfError::RateLimited { url: url_s },
            500..=599 => YfError::ServerError {
                status: code,
                url: url_s,
            },
            _ => YfError::Status {
                status: code,
                url: url_s,
            },
        });
    }

    let endpoint = format!("news_{}", tab_as_str(tab));
    let body = net::get_text(resp, &endpoint, symbol, "json").await?;
    let envelope: wire::NewsEnvelope = serde_json::from_str(&body).map_err(YfError::Json)?;

    let articles = envelope
        .data
        .and_then(|d| d.ticker_stream)
        .and_then(|ts| ts.stream)
        .unwrap_or_default();

    let results = articles
        .into_iter()
        .filter_map(|raw_item| {
            // Filter out ads or items that are not valid articles
            if raw_item.ad.is_some() {
                return None;
            }

            let content = raw_item.content?;
            let title = content.title?;
            let pub_date_str = content.pub_date?;

            // Parse the RFC3339 string to a timestamp
            let timestamp = chrono::DateTime::parse_from_rfc3339(&pub_date_str)
                .ok()?
                .timestamp();

            Some(NewsArticle {
                uuid: raw_item.id,
                title,
                publisher: content.provider.and_then(|p| p.display_name),
                link: content.canonical_url.and_then(|u| u.url),
                published_at: i64_to_datetime(timestamp),
            })
        })
        .collect();

    Ok(results)
}
