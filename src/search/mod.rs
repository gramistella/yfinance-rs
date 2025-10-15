use paft::domain::{AssetKind, Exchange};
use paft::market::responses::search::{SearchResponse, SearchResult};
use serde::Deserialize;
use url::Url;

use crate::core::client::CacheMode;
use crate::core::client::RetryConfig;
use crate::{YfClient, YfError};

fn parse_search_body(body: &str) -> Result<SearchResponse, YfError> {
    let env: V1SearchEnvelope = serde_json::from_str(body).map_err(YfError::Json)?;

    let quotes = env.quotes.unwrap_or_default();
    let results = quotes
        .into_iter()
        .map(|q| SearchResult {
            symbol: q.symbol.unwrap_or_default(),
            name: q.shortname.or(q.longname),
            exchange: q.exchange.and_then(|s| s.parse::<Exchange>().ok()),
            kind: q
                .quote_type
                .and_then(|s| s.parse::<AssetKind>().ok())
                .unwrap_or_default(),
        })
        .collect();

    Ok(SearchResponse { results })
}

/* ---------------- Public API ---------------- */

/// Searches for symbols matching a query.
///
/// # Errors
///
/// Returns `YfError` if the network request fails or the response cannot be parsed.
pub async fn search(client: &YfClient, query: &str) -> Result<SearchResponse, YfError> {
    SearchBuilder::new(client, query).fetch().await
}

/// A builder for searching for tickers and other assets on Yahoo Finance.
#[derive(Debug)]
pub struct SearchBuilder {
    client: YfClient,
    base: Url,
    query: String,
    quotes_count: Option<u32>,
    news_count: Option<u32>,
    lists_count: Option<u32>,
    lang: Option<String>,
    region: Option<String>,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl SearchBuilder {
    /// Creates a new `SearchBuilder` for a given search query.
    ///
    /// # Panics
    ///
    /// This function will panic if the hardcoded `DEFAULT_BASE_SEARCH_V1` constant
    /// is not a valid URL. This indicates a bug within the crate itself.
    pub fn new(client: &YfClient, query: impl Into<String>) -> Self {
        Self {
            client: client.clone(),
            base: Url::parse(DEFAULT_BASE_SEARCH_V1).unwrap(),
            query: query.into(),
            quotes_count: Some(10),
            news_count: Some(0),
            lists_count: Some(0),
            lang: None,
            region: None,
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for this specific API call.
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call.
    #[must_use]
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// (For testing) Overrides the base URL for the search API.
    #[must_use]
    pub fn search_base(mut self, base: Url) -> Self {
        self.base = base;
        self
    }

    /// Sets the maximum number of quote results to return.
    #[must_use]
    pub const fn quotes_count(mut self, n: u32) -> Self {
        self.quotes_count = Some(n);
        self
    }

    /// Sets the maximum number of news results to return.
    #[must_use]
    pub const fn news_count(mut self, n: u32) -> Self {
        self.news_count = Some(n);
        self
    }

    /// Sets the maximum number of screener list results to return.
    #[must_use]
    pub const fn lists_count(mut self, n: u32) -> Self {
        self.lists_count = Some(n);
        self
    }

    /// Sets the language for the search results.
    #[must_use]
    pub fn lang(mut self, s: impl Into<String>) -> Self {
        self.lang = Some(s.into());
        self
    }

    /// Sets the region for the search results.
    #[must_use]
    pub fn region(mut self, s: impl Into<String>) -> Self {
        self.region = Some(s.into());
        self
    }

    /// Executes the search request.
    ///
    /// # Errors
    ///
    /// This method will return an error if the network request fails, the API returns a
    /// non-successful status code, or the response body cannot be parsed as a valid search result.
    #[allow(clippy::too_many_lines)]
    pub async fn fetch(self) -> Result<SearchResponse, crate::core::YfError> {
        let mut url = self.base.clone();
        Self::append_query_params(
            &mut url,
            &self.query,
            self.quotes_count,
            self.news_count,
            self.lists_count,
            self.lang.as_deref(),
            self.region.as_deref(),
        );

        if self.cache_mode == CacheMode::Use
            && let Some(body) = self.client.cache_get(&url).await
        {
            return parse_search_body(&body);
        }

        let http = self.client.http().clone();
        let mut resp = self
            .client
            .send_with_retry(
                http.get(url.clone()).header("accept", "application/json"),
                self.retry_override.as_ref(),
            )
            .await?;

        if !resp.status().is_success() {
            let code = resp.status().as_u16();

            if code == 401 || code == 403 {
                self.client.ensure_credentials().await?;
                let crumb = self
                    .client
                    .crumb()
                    .await
                    .ok_or_else(|| crate::core::YfError::Auth("Crumb is not set".into()))?;

                let mut url2 = self.base.clone();
                Self::append_query_params(
                    &mut url2,
                    &self.query,
                    self.quotes_count,
                    self.news_count,
                    self.lists_count,
                    self.lang.as_deref(),
                    self.region.as_deref(),
                );
                url2.query_pairs_mut().append_pair("crumb", &crumb);

                resp = self
                    .client
                    .send_with_retry(
                        http.get(url2.clone()).header("accept", "application/json"),
                        self.retry_override.as_ref(),
                    )
                    .await?;

                if !resp.status().is_success() {
                    let code = resp.status().as_u16();
                    let url_s = url2.to_string();
                    return Err(match code {
                        404 => crate::core::YfError::NotFound { url: url_s },
                        429 => crate::core::YfError::RateLimited { url: url_s },
                        500..=599 => crate::core::YfError::ServerError {
                            status: code,
                            url: url_s,
                        },
                        _ => crate::core::YfError::Status {
                            status: code,
                            url: url_s,
                        },
                    });
                }

                let body =
                    crate::core::net::get_text(resp, "search_v1", &self.query, "json").await?;
                if self.cache_mode != CacheMode::Bypass {
                    self.client.cache_put(&url2, &body, None).await;
                }
                return parse_search_body(&body);
            }

            let url_s = url.to_string();
            return Err(match code {
                404 => crate::core::YfError::NotFound { url: url_s },
                429 => crate::core::YfError::RateLimited { url: url_s },
                500..=599 => crate::core::YfError::ServerError {
                    status: code,
                    url: url_s,
                },
                _ => crate::core::YfError::Status {
                    status: code,
                    url: url_s,
                },
            });
        }

        let body = crate::core::net::get_text(resp, "search_v1", &self.query, "json").await?;
        if self.cache_mode != CacheMode::Bypass {
            self.client.cache_put(&url, &body, None).await;
        }
        parse_search_body(&body)
    }

    fn append_query_params(
        url: &mut Url,
        query: &str,
        quotes_count: Option<u32>,
        news_count: Option<u32>,
        lists_count: Option<u32>,
        lang: Option<&str>,
        region: Option<&str>,
    ) {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("q", query);
        if let Some(n) = quotes_count {
            qp.append_pair("quotesCount", &n.to_string());
        }
        if let Some(n) = news_count {
            qp.append_pair("newsCount", &n.to_string());
        }
        if let Some(n) = lists_count {
            qp.append_pair("listsCount", &n.to_string());
        }
        if let Some(l) = lang {
            qp.append_pair("lang", l);
        }
        if let Some(r) = region {
            qp.append_pair("region", r);
        }
    }
}

/* ---------------- Types returned by this module ---------------- */
// Local types removed in favor of paft::market::responses::search::{SearchResponse, SearchResult}

const DEFAULT_BASE_SEARCH_V1: &str = "https://query2.finance.yahoo.com/v1/finance/search";

/* ------------- Minimal serde mapping of /v1/finance/search ------------- */

#[derive(Deserialize)]
struct V1SearchEnvelope {
    #[allow(dead_code)]
    explains: Option<serde_json::Value>,
    #[allow(dead_code)]
    count: Option<i64>,
    quotes: Option<Vec<V1SearchQuote>>,
    #[allow(dead_code)]
    news: Option<serde_json::Value>,
    #[allow(dead_code)]
    nav: Option<serde_json::Value>,
    #[allow(dead_code)]
    lists: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct V1SearchQuote {
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    shortname: Option<String>,
    #[serde(default)]
    longname: Option<String>,
    #[serde(rename = "quoteType")]
    #[serde(default)]
    quote_type: Option<String>,
    #[serde(default)]
    exchange: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "exchDisp")]
    #[serde(default)]
    exch_disp: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "typeDisp")]
    #[serde(default)]
    type_disp: Option<String>,
}
