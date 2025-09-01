use serde::Deserialize;
use serde::Serialize;
use url::Url;

use crate::core::client::CacheMode;
use crate::core::client::RetryConfig;
use crate::{YfClient, YfError};

fn parse_search_body(body: &str) -> Result<SearchResponse, YfError> {
    let env: V1SearchEnvelope = serde_json::from_str(body).map_err(|e| YfError::Json(e))?;

    let count = env.count.and_then(|c| u32::try_from(c).ok());
    let quotes = env.quotes.unwrap_or_default();

    let out = quotes
        .into_iter()
        .map(|q| SearchQuote {
            symbol: q.symbol.unwrap_or_default(),
            shortname: q.shortname,
            longname: q.longname,
            quote_type: q.quote_type,
            exchange: q.exchange,
            exch_disp: q.exch_disp,
            type_disp: q.type_disp,
        })
        .collect();

    Ok(SearchResponse { count, quotes: out })
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
    pub async fn fetch(self) -> Result<SearchResponse, crate::core::YfError> {
        let mut url = self.base.clone();
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("q", &self.query);
            if let Some(n) = self.quotes_count {
                qp.append_pair("quotesCount", &n.to_string());
            }
            if let Some(n) = self.news_count {
                qp.append_pair("newsCount", &n.to_string());
            }
            if let Some(n) = self.lists_count {
                qp.append_pair("listsCount", &n.to_string());
            }
            if let Some(l) = &self.lang {
                qp.append_pair("lang", l);
            }
            if let Some(r) = &self.region {
                qp.append_pair("region", r);
            }
        }

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
                {
                    let mut qp = url2.query_pairs_mut();
                    qp.append_pair("q", &self.query);
                    if let Some(n) = self.quotes_count {
                        qp.append_pair("quotesCount", &n.to_string());
                    }
                    if let Some(n) = self.news_count {
                        qp.append_pair("newsCount", &n.to_string());
                    }
                    if let Some(n) = self.lists_count {
                        qp.append_pair("listsCount", &n.to_string());
                    }
                    if let Some(l) = &self.lang {
                        qp.append_pair("lang", l);
                    }
                    if let Some(r) = &self.region {
                        qp.append_pair("region", r);
                    }
                    qp.append_pair("crumb", &crumb);
                }

                resp = self
                    .client
                    .send_with_retry(
                        http.get(url2.clone()).header("accept", "application/json"),
                        self.retry_override.as_ref(),
                    )
                    .await?;

                if !resp.status().is_success() {
                    return Err(crate::core::YfError::Status {
                        status: resp.status().as_u16(),
                        url: url2.to_string(),
                    });
                }

                let body =
                    crate::core::net::get_text(resp, "search_v1", &self.query, "json").await?;
                if self.cache_mode != CacheMode::Bypass {
                    self.client.cache_put(&url2, &body, None).await;
                }
                return parse_search_body(&body);
            }

            return Err(crate::core::YfError::Status {
                status: code,
                url: url.to_string(),
            });
        }

        let body = crate::core::net::get_text(resp, "search_v1", &self.query, "json").await?;
        if self.cache_mode != CacheMode::Bypass {
            self.client.cache_put(&url, &body, None).await;
        }
        parse_search_body(&body)
    }
}

/* ---------------- Types returned by this module ---------------- */

/// The response from a search query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchResponse {
    /// The total number of quote results found.
    pub count: Option<u32>,
    /// A list of quote results matching the query.
    pub quotes: Vec<SearchQuote>,
}

/// A quote result from a search query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchQuote {
    /// The ticker symbol.
    pub symbol: String,
    /// The short name of the company or asset.
    pub shortname: Option<String>,
    /// The long name of the company or asset.
    pub longname: Option<String>,
    /// The type of the quote (e.g., "EQUITY", "ETF").
    pub quote_type: Option<String>,
    /// The exchange the asset is traded on.
    pub exchange: Option<String>,
    /// The display name of the exchange.
    pub exch_disp: Option<String>,
    /// The display name of the asset type.
    pub type_disp: Option<String>,
}

const DEFAULT_BASE_SEARCH_V1: &str = "https://query2.finance.yahoo.com/v1/finance/search";

/* ------------- Minimal serde mapping of /v1/finance/search ------------- */

#[derive(Deserialize)]
struct V1SearchEnvelope {
    #[allow(dead_code)]
    explains: Option<serde_json::Value>,
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
    #[serde(rename = "exchDisp")]
    #[serde(default)]
    exch_disp: Option<String>,
    #[serde(rename = "typeDisp")]
    #[serde(default)]
    type_disp: Option<String>,
}
