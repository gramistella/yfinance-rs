//! Public client surface + builder.
//! Internals are split into `auth` (cookie/crumb) and `constants` (UA + defaults).

mod auth;
mod constants;
mod retry;

use crate::core::YfError;
pub use retry::{Backoff, CacheMode, RetryConfig};

use constants::{
    DEFAULT_BASE_CHART, DEFAULT_BASE_QUOTE, DEFAULT_BASE_QUOTE_API, DEFAULT_COOKIE_URL,
    DEFAULT_CRUMB_URL, USER_AGENT,
};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use url::Url;

#[cfg(feature = "test-mode")]
/// Defines the preferred data source for profile lookups when testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiPreference {
    /// Try the API first, then fall back to scraping if the API fails. (Default)
    ApiThenScrape,
    /// Use only the `quoteSummary` API.
    ApiOnly,
    /// Use only the HTML scraping method.
    ScrapeOnly,
}

#[derive(Debug)]
struct CacheEntry {
    body: String,
    expires_at: Instant,
}

#[derive(Debug)]
struct CacheStore {
    map: RwLock<HashMap<String, CacheEntry>>,
    default_ttl: Duration,
}

#[derive(Debug, Default)]
struct ClientState {
    cookie: Option<String>,
    crumb: Option<String>,
}

/// The main asynchronous client for interacting with the Yahoo Finance API.
///
/// The client manages an HTTP client, authentication (cookies and crumbs),
/// caching, and retry logic. It is cloneable and designed to be shared
/// across multiple tasks.
///
/// Create a client using [`YfClient::builder()`] or [`YfClient::default()`].
#[derive(Debug, Clone)]
pub struct YfClient {
    http: Client,
    base_chart: Url,
    base_quote: Url,
    base_quote_api: Url,
    cookie_url: Url,
    crumb_url: Url,
    user_agent: String,

    state: Arc<RwLock<ClientState>>,

    #[cfg(feature = "test-mode")]
    api_preference: ApiPreference,

    retry: RetryConfig,
    cache: Option<Arc<CacheStore>>,
}

impl Default for YfClient {
    fn default() -> Self {
        Self::builder().build().expect("default client")
    }
}

impl YfClient {
    /// Creates a new builder for a `YfClient`.
    pub fn builder() -> YfClientBuilder {
        YfClientBuilder::default()
    }

    /* -------- internal getters used by other modules -------- */

    pub(crate) fn http(&self) -> &Client {
        &self.http
    }

    pub(crate) fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub(crate) fn base_chart(&self) -> &Url {
        &self.base_chart
    }

    pub(crate) fn base_quote(&self) -> &Url {
        &self.base_quote
    }
    pub(crate) fn base_quote_api(&self) -> &Url {
        &self.base_quote_api
    }
    #[cfg(feature = "test-mode")]
    pub(crate) fn api_preference(&self) -> ApiPreference {
        self.api_preference
    }

    /// Returns `true` if in-memory caching is enabled for this client.
    pub fn cache_enabled(&self) -> bool {
        self.cache.is_some()
    }

    pub(crate) async fn cache_get(&self, url: &Url) -> Option<String> {
        let store = self.cache.as_ref()?;
        let key = url.as_str().to_string();
        let guard = store.map.read().await;
        if let Some(entry) = guard.get(&key)
            && Instant::now() <= entry.expires_at
        {
            return Some(entry.body.clone());
        }
        None
    }

    pub(crate) async fn cache_put(&self, url: &Url, body: &str, ttl_override: Option<Duration>) {
        let store = match &self.cache {
            Some(s) => s.clone(),
            None => return,
        };
        let key = url.as_str().to_string();
        let ttl = ttl_override.unwrap_or(store.default_ttl);
        let expires_at = Instant::now() + ttl;
        let entry = CacheEntry {
            body: body.to_string(),
            expires_at,
        };
        let mut guard = store.map.write().await;
        guard.insert(key, entry);
    }

    /// Clears the entire in-memory cache.
    ///
    /// This is an asynchronous operation that will acquire a write lock on the cache.
    /// It does nothing if caching is disabled for the client.
    pub async fn clear_cache(&self) {
        if let Some(store) = &self.cache {
            let mut guard = store.map.write().await;
            guard.clear();
        }
    }

    /// Removes a specific URL-based entry from the in-memory cache.
    ///
    /// This is useful if you know that the data for a specific request has become stale.
    /// It does nothing if caching is disabled for the client.
    pub async fn invalidate_cache_entry(&self, url: &Url) {
        if let Some(store) = &self.cache {
            let key = url.as_str().to_string();
            let mut guard = store.map.write().await;
            guard.remove(&key);
        }
    }

    pub(crate) async fn send_with_retry(
        &self,
        req: reqwest::RequestBuilder,
        override_retry: Option<&RetryConfig>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let cfg = override_retry.unwrap_or(&self.retry);
        if !cfg.enabled {
            return req.send().await;
        }

        let mut attempt = 0u32;
        loop {
            let res = req.try_clone().expect("cloneable request").send().await;

            match res {
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    if cfg.retry_on_status.contains(&code) && attempt < cfg.max_retries {
                        sleep_backoff(&cfg.backoff, attempt).await;
                        attempt += 1;
                        continue;
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    let should_retry = (cfg.retry_on_timeout && e.is_timeout())
                        || (cfg.retry_on_connect && e.is_connect());

                    if should_retry && attempt < cfg.max_retries {
                        sleep_backoff(&cfg.backoff, attempt).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Returns a reference to the default `RetryConfig` for this client.
    ///
    /// This config is used for all requests unless overridden on a per-call basis.
    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry
    }
}

/* ----------------------- Builder ----------------------- */

/// A builder for creating and configuring a [`YfClient`].
#[derive(Default)]
pub struct YfClientBuilder {
    user_agent: Option<String>,
    base_chart: Option<Url>,
    base_quote: Option<Url>,
    base_quote_api: Option<Url>,
    cookie_url: Option<Url>,
    crumb_url: Option<Url>,

    #[cfg(feature = "test-mode")]
    api_preference: Option<ApiPreference>,
    #[cfg(feature = "test-mode")]
    preauth_cookie: Option<String>,
    #[cfg(feature = "test-mode")]
    preauth_crumb: Option<String>,

    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    retry: Option<RetryConfig>,
    cache_ttl: Option<Duration>,
}

impl YfClientBuilder {
    /// Overrides the `User-Agent` header for all HTTP requests.
    ///
    /// Defaults to a common desktop browser User-Agent to avoid being blocked
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Overrides the base URL for quote HTML pages (used for scraping).
    /// Default: `https://finance.yahoo.com/quote/`.
    pub fn base_quote(mut self, url: Url) -> Self {
        self.base_quote = Some(url);
        self
    }

    /// Overrides the base URL for the chart API (used for historical data).
    /// Default: `https://query1.finance.yahoo.com/v8/finance/chart/`.
    pub fn base_chart(mut self, url: Url) -> Self {
        self.base_chart = Some(url);
        self
    }

    /// Overrides the base URL for the `quoteSummary` API (used for profiles, financials, etc.).
    /// Default: `https://query1.finance.yahoo.com/v10/finance/quoteSummary/`.
    pub fn base_quote_api(mut self, url: Url) -> Self {
        self.base_quote_api = Some(url);
        self
    }

    /// Overrides the URL used to acquire an initial cookie.
    pub fn cookie_url(mut self, url: Url) -> Self {
        self.cookie_url = Some(url);
        self
    }

    /// Overrides the URL used to acquire a crumb for authenticated requests.
    pub fn crumb_url(mut self, url: Url) -> Self {
        self.crumb_url = Some(url);
        self
    }

    /// Sets the entire retry configuration.
    ///
    /// Replaces the default retry settings.
    pub fn retry_config(mut self, cfg: RetryConfig) -> Self {
        self.retry = Some(cfg);
        self
    }

    /// A convenience method to enable or disable the retry mechanism.
    pub fn retry_enabled(mut self, yes: bool) -> Self {
        let mut cfg = self.retry.unwrap_or_default();
        cfg.enabled = yes;
        self.retry = Some(cfg);
        self
    }

    /// Disables in-memory caching for this client.
    pub fn no_cache(mut self) -> Self {
        self.cache_ttl = None;
        self
    }

    #[cfg(feature = "test-mode")]
    /// (Test mode only) Chooses which data source path to use for profile lookups.
    pub fn api_preference(mut self, pref: ApiPreference) -> Self {
        self.api_preference = Some(pref);
        self
    }

    #[cfg(feature = "test-mode")]
    /// (Test mode only) Provides pre-authenticated credentials to bypass the cookie/crumb fetch.
    pub fn preauth(mut self, cookie: impl Into<String>, crumb: impl Into<String>) -> Self {
        self.preauth_cookie = Some(cookie.into());
        self.preauth_crumb = Some(crumb.into());
        self
    }

    /// Sets a global timeout for the entire HTTP request.
    ///
    /// Default: none.
    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Sets a timeout for the connection phase of an HTTP request.
    ///
    /// Default: none.
    pub fn connect_timeout(mut self, dur: Duration) -> Self {
        self.connect_timeout = Some(dur);
        self
    }

    /// Enables in-memory caching with a default Time-To-Live (TTL) for all responses.
    ///
    /// If not set, caching is disabled by default.
    pub fn cache_ttl(mut self, dur: Duration) -> Self {
        self.cache_ttl = Some(dur);
        self
    }
    /// Builds the `YfClient`.
    pub fn build(self) -> Result<YfClient, YfError> {
        let base_chart = self.base_chart.unwrap_or(Url::parse(DEFAULT_BASE_CHART)?);
        let base_quote = self.base_quote.unwrap_or(Url::parse(DEFAULT_BASE_QUOTE)?);
        let base_quote_api = self
            .base_quote_api
            .unwrap_or(Url::parse(DEFAULT_BASE_QUOTE_API)?);
        let cookie_url = self.cookie_url.unwrap_or(Url::parse(DEFAULT_COOKIE_URL)?);
        let crumb_url = self.crumb_url.unwrap_or(Url::parse(DEFAULT_CRUMB_URL)?);

        let user_agent = self.user_agent.as_deref().unwrap_or(USER_AGENT).to_string();
        let mut httpb = reqwest::Client::builder()
            .user_agent(user_agent.clone())
            .cookie_store(true);

        if let Some(t) = self.timeout {
            httpb = httpb.timeout(t);
        }
        if let Some(ct) = self.connect_timeout {
            httpb = httpb.connect_timeout(ct);
        }

        let http = httpb.build()?;

        let initial_state = ClientState {
            cookie: {
                #[cfg(feature = "test-mode")]
                {
                    self.preauth_cookie
                }
                #[cfg(not(feature = "test-mode"))]
                {
                    None
                }
            },
            crumb: {
                #[cfg(feature = "test-mode")]
                {
                    self.preauth_crumb
                }
                #[cfg(not(feature = "test-mode"))]
                {
                    None
                }
            },
        };

        Ok(YfClient {
            http,
            base_chart,
            base_quote,
            base_quote_api,
            cookie_url,
            crumb_url,
            user_agent,
            state: Arc::new(RwLock::new(initial_state)),
            #[cfg(feature = "test-mode")]
            api_preference: self.api_preference.unwrap_or(ApiPreference::ApiThenScrape),
            retry: self.retry.unwrap_or_default(),
            cache: self.cache_ttl.map(|ttl| {
                Arc::new(CacheStore {
                    map: RwLock::new(HashMap::new()),
                    default_ttl: ttl,
                })
            }),
        })
    }
}

async fn sleep_backoff(b: &Backoff, attempt: u32) {
    use std::time::Duration;
    let dur = match *b {
        Backoff::Fixed(d) => d,
        Backoff::Exponential {
            base,
            factor,
            max,
            jitter,
        } => {
            let pow = factor.powi(attempt as i32);
            let mut d = Duration::from_secs_f64(base.as_secs_f64() * pow);
            if d > max {
                d = max;
            }
            if jitter {
                // simple +/- 50% jitter without extra deps
                let nanos = d.as_nanos();
                let j = ((nanos / 2) as u64) * ((attempt as u64 % 5 + 1) * 13 % 100) / 100;
                let sign = attempt % 2 == 0;
                d = if sign {
                    d.saturating_add(Duration::from_nanos(j))
                } else {
                    d.saturating_sub(Duration::from_nanos(j))
                };
            }
            d
        }
    };
    tokio::time::sleep(dur).await;
}
