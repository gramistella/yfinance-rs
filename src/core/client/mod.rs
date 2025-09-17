//! Public client surface + builder.
//! Internals are split into `auth` (cookie/crumb) and `constants` (UA + defaults).

mod auth;
mod constants;
mod retry;

use crate::core::YfError;
use crate::core::client::constants::DEFAULT_BASE_INSIDER_SEARCH;
use crate::core::currency::currency_for_country;
use paft::prelude::Currency;
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

/// Defines the preferred data source for profile lookups when testing.
///
/// This enum is always available for API compatibility, but only has effect when
/// the `test-mode` feature is enabled.
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
    base_quote_v7: Url,
    base_options_v7: Url,
    base_stream: Url,
    base_news: Url,
    base_insider_search: Url,
    base_timeseries: Url,
    cookie_url: Url,
    crumb_url: Url,
    user_agent: String,

    state: Arc<RwLock<ClientState>>,
    credential_fetch_lock: Arc<tokio::sync::Mutex<()>>,

    #[cfg(feature = "test-mode")]
    api_preference: ApiPreference,

    retry: RetryConfig,
    reporting_currency_cache: Arc<RwLock<HashMap<String, Currency>>>,
    cache: Option<Arc<CacheStore>>,
}

impl Default for YfClient {
    fn default() -> Self {
        Self::builder().build().expect("default client")
    }
}

impl YfClient {
    /// Creates a new builder for a `YfClient`.
    #[must_use]
    pub fn builder() -> YfClientBuilder {
        YfClientBuilder::default()
    }

    /* -------- internal getters used by other modules -------- */

    pub(crate) const fn http(&self) -> &Client {
        &self.http
    }

    pub(crate) fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub(crate) const fn base_chart(&self) -> &Url {
        &self.base_chart
    }

    pub(crate) const fn base_quote(&self) -> &Url {
        &self.base_quote
    }
    pub(crate) const fn base_quote_api(&self) -> &Url {
        &self.base_quote_api
    }

    pub(crate) const fn base_quote_v7(&self) -> &Url {
        &self.base_quote_v7
    }

    pub(crate) const fn base_options_v7(&self) -> &Url {
        &self.base_options_v7
    }

    pub(crate) const fn base_stream(&self) -> &Url {
        &self.base_stream
    }

    pub(crate) const fn base_news(&self) -> &Url {
        &self.base_news
    }

    pub(crate) const fn base_insider_search(&self) -> &Url {
        &self.base_insider_search
    }

    pub(crate) const fn base_timeseries(&self) -> &Url {
        &self.base_timeseries
    }

    #[cfg(feature = "test-mode")]
    pub(crate) const fn api_preference(&self) -> ApiPreference {
        self.api_preference
    }

    /// Returns `true` if in-memory caching is enabled for this client.
    #[must_use]
    pub const fn cache_enabled(&self) -> bool {
        self.cache.is_some()
    }

    pub(crate) async fn cache_get(&self, url: &Url) -> Option<String> {
        let store = self.cache.as_ref()?;
        let key = url.as_str().to_string();
        if let Some(entry) = store.map.read().await.get(&key)
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

    async fn cached_reporting_currency(&self, symbol: &str) -> Option<Currency> {
        let guard = self.reporting_currency_cache.read().await;
        guard.get(symbol).cloned()
    }

    async fn store_reporting_currency(&self, symbol: &str, currency: Currency) {
        let mut guard = self.reporting_currency_cache.write().await;
        guard.insert(symbol.to_string(), currency);
    }

    /// Returns the cached or inferred reporting currency for a symbol.
    pub(crate) async fn reporting_currency(
        &self,
        symbol: &str,
        override_currency: Option<Currency>,
    ) -> Currency {
        if let Some(currency) = override_currency {
            self.store_reporting_currency(symbol, currency.clone())
                .await;
            return currency;
        }

        if let Some(currency) = self.cached_reporting_currency(symbol).await {
            return currency;
        }

        let mut debug_reason: Option<String> = None;
        let currency = match crate::profile::load_profile(self, symbol).await {
            Ok(profile) => extract_currency_from_profile(&profile).map_or_else(
                || {
                    debug_reason = Some("profile missing country or unsupported currency".into());
                    Currency::USD
                },
                |currency| currency,
            ),
            Err(err) => {
                debug_reason = Some(format!("failed to load profile: {err}"));
                Currency::USD
            }
        };

        if let Some(reason) =
            debug_reason.filter(|_| std::env::var("YF_DEBUG").ok().as_deref() == Some("1"))
        {
            eprintln!(
                "YF_DEBUG(currency): {symbol} -> {reason}; using {}",
                currency.code()
            );
        }

        self.store_reporting_currency(symbol, currency.clone())
            .await;
        currency
    }

    pub(crate) async fn send_with_retry(
        &self,
        mut req: reqwest::RequestBuilder,
        override_retry: Option<&RetryConfig>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        // Always set User-Agent header explicitly
        req = req.header("User-Agent", &self.user_agent);

        let cfg = override_retry.unwrap_or(&self.retry);
        if !cfg.enabled {
            return req.send().await;
        }

        let mut attempt = 0u32;
        loop {
            let response = req.try_clone().expect("cloneable request").send().await;

            match response {
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
    #[must_use]
    pub const fn retry_config(&self) -> &RetryConfig {
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
    base_quote_v7: Option<Url>,
    base_options_v7: Option<Url>,
    base_stream: Option<Url>,
    base_news: Option<Url>,
    base_insider_search: Option<Url>,
    base_timeseries: Option<Url>,
    cookie_url: Option<Url>,
    crumb_url: Option<Url>,

    #[allow(dead_code)]
    api_preference: Option<ApiPreference>,
    #[allow(dead_code)]
    preauth_cookie: Option<String>,
    #[allow(dead_code)]
    preauth_crumb: Option<String>,

    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    retry: Option<RetryConfig>,
    cache_ttl: Option<Duration>,

    // New fields for custom client and proxy configuration
    custom_client: Option<Client>,
    proxy: Option<reqwest::Proxy>,
}

fn extract_currency_from_profile(profile: &crate::profile::Profile) -> Option<Currency> {
    match profile {
        crate::profile::Profile::Company(company) => company
            .address
            .as_ref()
            .and_then(|addr| addr.country.as_deref())
            .and_then(currency_for_country),
        crate::profile::Profile::Fund(_) => None,
    }
}

impl YfClientBuilder {
    /// Sets the `User-Agent` header for all HTTP requests and WebSocket connections.
    ///
    /// The user agent is applied consistently across all request types:
    /// - HTTP requests (quotes, history, fundamentals, etc.)
    /// - WebSocket streaming connections
    /// - Authentication requests (cookies, crumbs)
    ///
    /// Defaults to a common desktop browser User-Agent to avoid being blocked.
    /// This setting is applied per-request rather than at the HTTP client level.
    #[must_use]
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Overrides the base URL for quote HTML pages (used for scraping).
    /// Default: `https://finance.yahoo.com/quote/`.
    #[must_use]
    pub fn base_quote(mut self, url: Url) -> Self {
        self.base_quote = Some(url);
        self
    }

    /// Overrides the base URL for the chart API (used for historical data).
    /// Default: `https://query1.finance.yahoo.com/v8/finance/chart/`.
    #[must_use]
    pub fn base_chart(mut self, url: Url) -> Self {
        self.base_chart = Some(url);
        self
    }

    /// Overrides the base URL for the `quoteSummary` API (used for profiles, financials, etc.).
    /// Default: `https://query1.finance.yahoo.com/v10/finance/quoteSummary/`.
    #[must_use]
    pub fn base_quote_api(mut self, url: Url) -> Self {
        self.base_quote_api = Some(url);
        self
    }

    /// Sets a custom base URL for the v7 quote endpoint.
    ///
    /// This is primarily used for testing or to target a different Yahoo Finance region.
    /// If not set, a default URL (`https://query1.finance.yahoo.com/v7/finance/quote`) is used.
    #[must_use]
    pub fn base_quote_v7(mut self, url: Url) -> Self {
        self.base_quote_v7 = Some(url);
        self
    }

    /// Sets a custom base URL for the v7 options endpoint.
    ///
    /// This is primarily used for testing or to target a different Yahoo Finance region.
    /// If not set, a default URL (`https://query1.finance.yahoo.com/v7/finance/options/`) is used.
    #[must_use]
    pub fn base_options_v7(mut self, url: Url) -> Self {
        self.base_options_v7 = Some(url);
        self
    }

    /// Sets a custom base URL for the streaming API.
    #[must_use]
    pub fn base_stream(mut self, url: Url) -> Self {
        self.base_stream = Some(url);
        self
    }

    /// Sets a custom base URL for the news endpoint.
    /// Default: `https://finance.yahoo.com`.
    #[must_use]
    pub fn base_news(mut self, url: Url) -> Self {
        self.base_news = Some(url);
        self
    }

    /// Sets a custom base URL for the Business Insider search (for ISIN lookup).
    #[must_use]
    pub fn base_insider_search(mut self, url: Url) -> Self {
        self.base_insider_search = Some(url);
        self
    }

    /// Sets a custom base URL for the timeseries endpoint.
    #[must_use]
    pub fn base_timeseries(mut self, url: Url) -> Self {
        self.base_timeseries = Some(url);
        self
    }

    /// Overrides the URL used to acquire an initial cookie.
    #[must_use]
    pub fn cookie_url(mut self, url: Url) -> Self {
        self.cookie_url = Some(url);
        self
    }

    /// Overrides the URL used to acquire a crumb for authenticated requests.
    #[must_use]
    pub fn crumb_url(mut self, url: Url) -> Self {
        self.crumb_url = Some(url);
        self
    }

    /// Sets the entire retry configuration.
    ///
    /// Replaces the default retry settings.
    #[must_use]
    pub fn retry_config(mut self, cfg: RetryConfig) -> Self {
        self.retry = Some(cfg);
        self
    }

    /// A convenience method to enable or disable the retry mechanism.
    #[must_use]
    pub fn retry_enabled(mut self, yes: bool) -> Self {
        let mut cfg = self.retry.unwrap_or_default();
        cfg.enabled = yes;
        self.retry = Some(cfg);
        self
    }

    /// Disables in-memory caching for this client.
    #[must_use]
    pub const fn no_cache(mut self) -> Self {
        self.cache_ttl = None;
        self
    }

    /// (Internal testing only) Chooses which data source path to use for profile lookups.
    ///
    /// This setting only has effect when the `test-mode` feature is enabled.
    /// In normal usage, this setting is ignored.
    #[doc(hidden)]
    #[must_use]
    #[allow(unused_variables, unused_mut)]
    pub const fn _api_preference(mut self, pref: ApiPreference) -> Self {
        #[cfg(feature = "test-mode")]
        {
            self.api_preference = Some(pref);
        }
        self
    }

    /// (Internal testing only) Provides pre-authenticated credentials to bypass the cookie/crumb fetch.
    ///
    /// This setting only has effect when the `test-mode` feature is enabled.
    /// In normal usage, this setting is ignored.
    #[doc(hidden)]
    #[must_use]
    #[allow(unused_variables, unused_mut)]
    pub fn _preauth(mut self, cookie: impl Into<String>, crumb: impl Into<String>) -> Self {
        #[cfg(feature = "test-mode")]
        {
            self.preauth_cookie = Some(cookie.into());
            self.preauth_crumb = Some(crumb.into());
        }
        self
    }

    /// Sets a global timeout for the entire HTTP request.
    ///
    /// Default: none.
    #[must_use]
    pub const fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Sets a timeout for the connection phase of an HTTP request.
    ///
    /// Default: none.
    #[must_use]
    pub const fn connect_timeout(mut self, dur: Duration) -> Self {
        self.connect_timeout = Some(dur);
        self
    }

    /// Enables in-memory caching with a default Time-To-Live (TTL) for all responses.
    ///
    /// If not set, caching is disabled by default.
    #[must_use]
    pub const fn cache_ttl(mut self, dur: Duration) -> Self {
        self.cache_ttl = Some(dur);
        self
    }

    /// Sets a custom reqwest client for full control over HTTP configuration.
    ///
    /// This allows you to configure advanced features like custom TLS settings,
    /// connection pooling, or other reqwest-specific options. When this is set,
    /// other HTTP-related builder methods (timeout, `connect_timeout`, proxy) are ignored.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reqwest::Client;
    /// use yfinance_rs::YfClient;
    ///
    /// let custom_client = Client::builder()
    ///     .timeout(std::time::Duration::from_secs(30))
    ///     .build()
    ///     .unwrap();
    ///
    /// let client = YfClient::builder()
    ///     .custom_client(custom_client)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn custom_client(mut self, client: Client) -> Self {
        self.custom_client = Some(client);
        self
    }

    /// Sets an HTTP proxy for all requests.
    ///
    /// This is a convenience method for setting up proxy configuration without
    /// needing to create a full custom client. If you need more advanced proxy
    /// configuration, use `custom_client()` instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use yfinance_rs::YfClient;
    ///
    /// let client = YfClient::builder()
    ///     .proxy("http://proxy.example.com:8080")
    ///     .build()
    ///     .unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// This method will panic if the proxy URL is invalid. For production code,
    /// consider using `try_proxy()` instead.
    ///
    /// # Panics
    ///
    /// Panics if the proxy URL format is invalid.
    #[must_use]
    pub fn proxy(mut self, proxy_url: &str) -> Self {
        // Validate URL format before creating proxy
        assert!(
            url::Url::parse(proxy_url).is_ok(),
            "invalid proxy URL format: {proxy_url}"
        );
        self.proxy = Some(reqwest::Proxy::http(proxy_url).expect("invalid proxy URL"));
        self
    }

    /// Sets an HTTP proxy for all requests with error handling.
    ///
    /// This is a convenience method for setting up proxy configuration without
    /// needing to create a full custom client. If you need more advanced proxy
    /// configuration, use `custom_client()` instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use yfinance_rs::YfClient;
    ///
    /// let client = YfClient::builder()
    ///     .try_proxy("http://proxy.example.com:8080")?
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the proxy URL is invalid.
    pub fn try_proxy(mut self, proxy_url: &str) -> Result<Self, YfError> {
        // Validate URL format first
        url::Url::parse(proxy_url)
            .map_err(|e| YfError::InvalidParams(format!("invalid proxy URL format: {e}")))?;

        let proxy = reqwest::Proxy::http(proxy_url)
            .map_err(|e| YfError::InvalidParams(format!("invalid proxy URL: {e}")))?;
        self.proxy = Some(proxy);
        Ok(self)
    }

    /// Sets an HTTPS proxy for all requests.
    ///
    /// This is a convenience method for setting up HTTPS proxy configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use yfinance_rs::YfClient;
    ///
    /// let client = YfClient::builder()
    ///     .https_proxy("https://proxy.example.com:8443")
    ///     .build()
    ///     .unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// This method will panic if the proxy URL is invalid. For production code,
    /// consider using `try_https_proxy()` instead.
    ///
    /// # Panics
    ///
    /// Panics if the proxy URL format is invalid.
    #[must_use]
    pub fn https_proxy(mut self, proxy_url: &str) -> Self {
        // Validate URL format before creating proxy
        assert!(
            url::Url::parse(proxy_url).is_ok(),
            "invalid HTTPS proxy URL format: {proxy_url}"
        );
        self.proxy = Some(reqwest::Proxy::https(proxy_url).expect("invalid HTTPS proxy URL"));
        self
    }

    /// Sets an HTTPS proxy for all requests with error handling.
    ///
    /// This is a convenience method for setting up HTTPS proxy configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use yfinance_rs::YfClient;
    ///
    /// let client = YfClient::builder()
    ///     .try_https_proxy("https://proxy.example.com:8443")?
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the proxy URL is invalid.
    pub fn try_https_proxy(mut self, proxy_url: &str) -> Result<Self, YfError> {
        // Validate URL format first
        url::Url::parse(proxy_url)
            .map_err(|e| YfError::InvalidParams(format!("invalid HTTPS proxy URL format: {e}")))?;

        let proxy = reqwest::Proxy::https(proxy_url)
            .map_err(|e| YfError::InvalidParams(format!("invalid HTTPS proxy URL: {e}")))?;
        self.proxy = Some(proxy);
        Ok(self)
    }

    /// Builds the `YfClient`.
    ///
    /// # Errors
    ///
    /// Returns an error if the base URLs are invalid or the HTTP client fails to build.
    pub fn build(self) -> Result<YfClient, YfError> {
        let base_chart = self.base_chart.unwrap_or(Url::parse(DEFAULT_BASE_CHART)?);
        let base_quote = self.base_quote.unwrap_or(Url::parse(DEFAULT_BASE_QUOTE)?);
        let base_quote_api = self
            .base_quote_api
            .unwrap_or(Url::parse(DEFAULT_BASE_QUOTE_API)?);
        let base_quote_v7 = self
            .base_quote_v7
            .unwrap_or(Url::parse(constants::DEFAULT_BASE_QUOTE_V7)?);
        let base_options_v7 = self
            .base_options_v7
            .unwrap_or(Url::parse(constants::DEFAULT_BASE_OPTIONS_V7)?);
        let base_stream = self
            .base_stream
            .unwrap_or(Url::parse(constants::DEFAULT_BASE_STREAM)?);
        let base_news = self
            .base_news
            .unwrap_or(Url::parse(constants::DEFAULT_BASE_NEWS)?);
        let base_insider_search = self
            .base_insider_search
            .unwrap_or(Url::parse(DEFAULT_BASE_INSIDER_SEARCH)?);
        let base_timeseries = self
            .base_timeseries
            .unwrap_or(Url::parse(constants::DEFAULT_BASE_TIMESERIES)?);

        let cookie_url = self.cookie_url.unwrap_or(Url::parse(DEFAULT_COOKIE_URL)?);
        let crumb_url = self.crumb_url.unwrap_or(Url::parse(DEFAULT_CRUMB_URL)?);

        let user_agent = self.user_agent.as_deref().unwrap_or(USER_AGENT).to_string();

        // Use custom client if provided, otherwise build a new one
        let http = if let Some(custom_client) = self.custom_client {
            custom_client
        } else {
            let mut httpb = reqwest::Client::builder().cookie_store(true);

            if let Some(t) = self.timeout {
                httpb = httpb.timeout(t);
            }
            if let Some(ct) = self.connect_timeout {
                httpb = httpb.connect_timeout(ct);
            }
            if let Some(proxy) = self.proxy {
                httpb = httpb.proxy(proxy);
            }

            httpb.build()?
        };

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
            base_quote_v7,
            base_options_v7,
            base_stream,
            base_news,
            base_insider_search,
            base_timeseries,
            cookie_url,
            crumb_url,
            user_agent,
            state: Arc::new(RwLock::new(initial_state)),
            credential_fetch_lock: Arc::new(tokio::sync::Mutex::new(())),
            #[cfg(feature = "test-mode")]
            api_preference: self.api_preference.unwrap_or(ApiPreference::ApiThenScrape),
            retry: self.retry.unwrap_or_default(),
            reporting_currency_cache: Arc::new(RwLock::new(HashMap::new())),
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
            let pow = factor.powi(i32::try_from(attempt).unwrap());
            let mut d = Duration::from_secs_f64(base.as_secs_f64() * pow);
            if d > max {
                d = max;
            }
            if jitter {
                // simple +/- 50% jitter without extra deps
                let nanos = d.as_nanos();
                let j = u64::try_from(nanos / 2).unwrap_or(0)
                    * ((u64::from(attempt) % 5 + 1) * 13 % 100)
                    / 100;
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
