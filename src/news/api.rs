use serde::Serialize;
use serde_json::Value;

use crate::{
    core::{
        CallOptions, ProjectionContext, ProjectionIssue, YfClient, YfError,
        client::{CacheEndpoint, normalize_symbol},
        diagnostics::{optional_wire_cloned, optional_wire_value, required_wire_value},
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

#[allow(clippy::too_many_lines)]
pub(super) async fn fetch_news(
    client: &YfClient,
    symbol: &str,
    count: u32,
    tab: NewsTab,
    options: &CallOptions,
) -> Result<crate::YfResponse<Vec<NewsArticle>>, YfError> {
    let symbol = normalize_symbol(symbol)?;
    let mut ctx = ProjectionContext::new("news", options.data_quality());
    let mut url = client.base_news().join("xhr/ncp").map_err(YfError::url)?;
    url.query_pairs_mut()
        .append_pair("queryRef", tab_as_str(tab))
        .append_pair("serviceKey", "ncp_fin");

    let symbols = [symbol.as_str()];
    let payload = NewsPayload {
        service_config: ServiceConfig {
            snippet_count: count,
            s: &symbols,
        },
    };

    let endpoint = format!("news_{}", tab_as_str(tab));
    let body_json = serde_json::to_string(&payload).map_err(YfError::json)?;
    let envelope: wire::NewsEnvelope = net::fetch_json_post_cached(
        client,
        &url,
        &body_json,
        net::CacheFetchConfig {
            cache_endpoint: CacheEndpoint::News,
            options,
            endpoint: &endpoint,
            fixture_key: &symbol,
            ext: "json",
            cache_validator: None,
        },
    )
    .await?;

    let articles = envelope
        .data
        .and_then(|d| d.ticker_stream)
        .and_then(|ts| ts.stream)
        .unwrap_or_default();

    let mut results = Vec::new();
    for (idx, raw_item) in articles.into_iter().enumerate() {
        let raw_key = Some(news_stream_item_diag_key(&raw_item, idx));
        let raw_item = match serde_json::from_value::<wire::StreamItem>(raw_item) {
            Ok(raw_item) => raw_item,
            Err(err) => {
                ctx.dropped_item(
                    "news_article",
                    raw_key.as_deref(),
                    ProjectionIssue::InvalidField {
                        field: "article",
                        details: err.to_string(),
                    },
                )?;
                continue;
            }
        };
        if raw_item.ad.is_some() {
            ctx.dropped_item(
                "news_article",
                raw_key.as_deref(),
                ProjectionIssue::ProviderUnavailable { feature: "article" },
            )?;
            continue;
        }

        let Some(id) = required_wire_value(
            &mut ctx,
            "news_article",
            raw_key.as_deref(),
            "id",
            &raw_item.id,
        )?
        .map(|id| id.trim().to_string()) else {
            continue;
        };
        if id.is_empty() {
            ctx.dropped_item(
                "news_article",
                raw_key.as_deref(),
                ProjectionIssue::MissingRequiredField { field: "id" },
            )?;
            continue;
        }
        let key = Some(id.as_str());

        let Some(content) =
            required_wire_value(&mut ctx, "news_article", key, "content", &raw_item.content)?
        else {
            continue;
        };
        let Some(title) =
            required_wire_value(&mut ctx, "news_article", key, "title", &content.title)?.cloned()
        else {
            continue;
        };
        let Some(pub_date_str) =
            required_wire_value(&mut ctx, "news_article", key, "pubDate", &content.pub_date)?
        else {
            continue;
        };

        let published_at = match chrono::DateTime::parse_from_rfc3339(pub_date_str) {
            Ok(date) => date.with_timezone(&chrono::Utc),
            Err(err) => {
                ctx.dropped_item(
                    "news_article",
                    key,
                    ProjectionIssue::InvalidField {
                        field: "pubDate",
                        details: err.to_string(),
                    },
                )?;
                continue;
            }
        };
        let provider =
            optional_wire_value(&mut ctx, "provider", key, "provider", &content.provider)?;
        let publisher = if let Some(provider) = provider {
            optional_wire_cloned(
                &mut ctx,
                "provider.displayName",
                key,
                "displayName",
                &provider.display_name,
            )?
        } else {
            None
        };
        let canonical_url = optional_wire_value(
            &mut ctx,
            "canonicalUrl",
            key,
            "canonicalUrl",
            &content.canonical_url,
        )?;
        let link = if let Some(canonical_url) = canonical_url {
            optional_wire_cloned(&mut ctx, "canonicalUrl.url", key, "url", &canonical_url.url)?
        } else {
            None
        };

        results.push(NewsArticle {
            uuid: id,
            title,
            publisher,
            link,
            published_at,
            provider: (),
        });
    }

    Ok(ctx.finish(results))
}

fn news_stream_item_diag_key(value: &Value, idx: usize) -> String {
    value
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map_or_else(|| format!("stream[{idx}]"), ToString::to_string)
}
