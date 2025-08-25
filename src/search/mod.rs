use serde::Deserialize;
use serde::Serialize;
use url::Url;

use crate::core::client::CacheMode;
use crate::core::client::RetryConfig;
use crate::{YfClient, YfError};

async fn parse_search_body(body: &str) -> Result<SearchResponse, YfError> {
    let env: V1SearchEnvelope =
        serde_json::from_str(body).map_err(|e| YfError::Data(format!("search json parse: {e}")))?;

    let count = env.count.map(|c| c as u32);
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

/// Convenience: perform a search with default settings (quotes only).
pub async fn search(client: &mut YfClient, query: &str) -> Result<SearchResponse, YfError> {
    SearchBuilder::new(client, query)?.fetch().await
}

#[derive(Debug)]
pub struct SearchBuilder<'a> {
    client: &'a mut YfClient,
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

impl<'a> SearchBuilder<'a> {
    pub fn new(client: &'a mut YfClient, query: impl Into<String>) -> Result<Self, YfError> {
        Ok(Self {
            client,
            base: Url::parse(DEFAULT_BASE_SEARCH_V1)?,
            query: query.into(),
            quotes_count: Some(10),
            news_count: Some(0),
            lists_count: Some(0),
            lang: None,
            region: None,
            cache_mode: CacheMode::Use,
            retry_override: None,
        })
    }

    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Override the base URL (useful for tests/mocking).
    pub fn search_base(mut self, base: Url) -> Self {
        self.base = base;
        self
    }

    pub fn quotes_count(mut self, n: u32) -> Self {
        self.quotes_count = Some(n);
        self
    }

    pub fn news_count(mut self, n: u32) -> Self {
        self.news_count = Some(n);
        self
    }

    pub fn lists_count(mut self, n: u32) -> Self {
        self.lists_count = Some(n);
        self
    }

    pub fn lang(mut self, s: impl Into<String>) -> Self {
        self.lang = Some(s.into());
        self
    }

    pub fn region(mut self, s: impl Into<String>) -> Self {
        self.region = Some(s.into());
        self
    }

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
            return parse_search_body(&body).await;
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
                    .ok_or_else(|| crate::core::YfError::Status {
                        status: code,
                        url: url.to_string(),
                    })?
                    .to_string();

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
                return parse_search_body(&body).await;
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
        parse_search_body(&body).await
    }
}

/* ---------------- Types returned by this module ---------------- */

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchResponse {
    pub count: Option<u32>,
    pub quotes: Vec<SearchQuote>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchQuote {
    pub symbol: String,
    pub shortname: Option<String>,
    pub longname: Option<String>,
    pub quote_type: Option<String>,
    pub exchange: Option<String>,
    pub exch_disp: Option<String>,
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
