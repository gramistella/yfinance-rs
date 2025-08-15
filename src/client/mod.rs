//! Public client surface + builder.
//! Internals are split into `auth` (cookie/crumb) and `constants` (UA + defaults).

mod auth;
mod constants;

use crate::error::YfError;
use constants::{
    DEFAULT_BASE_CHART, DEFAULT_BASE_QUOTE, DEFAULT_BASE_QUOTE_API, DEFAULT_COOKIE_URL,
    DEFAULT_CRUMB_URL, USER_AGENT,
};
use reqwest::Client;
use url::Url;

#[cfg(feature = "test-mode")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiPreference {
    /// Try API first, then fall back to scraping if API fails.
    ApiThenScrape,
    /// Use only the API path.
    ApiOnly,
    /// Use only the scraping path.
    ScrapeOnly,
}

#[derive(Clone)]
pub struct YfClient {
    http: Client,
    base_chart: Url,
    base_quote: Url,
    base_quote_api: Url,
    cookie_url: Url,
    crumb_url: Url,

    cookie: Option<String>,
    crumb: Option<String>,

    #[cfg(feature = "test-mode")]
    api_preference: ApiPreference,
}

impl Default for YfClient {
    fn default() -> Self {
        Self::builder().build().expect("default client")
    }
}

impl YfClient {
    /// Create a new builder.
    pub fn builder() -> YfClientBuilder {
        YfClientBuilder::default()
    }

    /* -------- internal getters used by other modules -------- */

    pub(crate) fn http(&self) -> &Client {
        &self.http
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
}

/* ----------------------- Builder ----------------------- */

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
}

impl YfClientBuilder {
    /// Override the User-Agent.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Override the quotes HTML base (e.g., `https://finance.yahoo.com/quote/`).
    pub fn base_quote(mut self, url: Url) -> Self {
        self.base_quote = Some(url);
        self
    }

    /// Override the chart API base (e.g., `https://query1.finance.yahoo.com/v8/finance/chart/`).
    pub fn base_chart(mut self, url: Url) -> Self {
        self.base_chart = Some(url);
        self
    }

    /// Override the quoteSummary API base (e.g., `https://query1.finance.yahoo.com/v10/finance/quoteSummary/`).
    pub fn base_quote_api(mut self, url: Url) -> Self {
        self.base_quote_api = Some(url);
        self
    }

    /// Override the cookie bootstrap URL.
    pub fn cookie_url(mut self, url: Url) -> Self {
        self.cookie_url = Some(url);
        self
    }

    /// Override the crumb URL.
    pub fn crumb_url(mut self, url: Url) -> Self {
        self.crumb_url = Some(url);
        self
    }

    #[cfg(feature = "test-mode")]
    /// Choose which data source path to use in tests.
    pub fn api_preference(mut self, pref: ApiPreference) -> Self {
        self.api_preference = Some(pref);
        self
    }

    #[cfg(feature = "test-mode")]
    /// Provide pre-auth credentials (bypass cookie/crumb fetch) in tests.
    pub fn preauth(mut self, cookie: impl Into<String>, crumb: impl Into<String>) -> Self {
        self.preauth_cookie = Some(cookie.into());
        self.preauth_crumb = Some(crumb.into());
        self
    }

    pub fn build(self) -> Result<YfClient, YfError> {
        let base_chart = self
            .base_chart
            .unwrap_or(Url::parse(DEFAULT_BASE_CHART)?);

        let base_quote = self
            .base_quote
            .unwrap_or(Url::parse(DEFAULT_BASE_QUOTE)?);

        let base_quote_api = self
            .base_quote_api
            .unwrap_or(Url::parse(DEFAULT_BASE_QUOTE_API)?);

        let cookie_url = self
            .cookie_url
            .unwrap_or(Url::parse(DEFAULT_COOKIE_URL)?);

        let crumb_url = self
            .crumb_url
            .unwrap_or(Url::parse(DEFAULT_CRUMB_URL)?);

        let http = reqwest::Client::builder()
            .user_agent(self.user_agent.as_deref().unwrap_or(USER_AGENT))
            .cookie_store(true)
            .build()?;

        Ok(YfClient {
            http,
            base_chart,
            base_quote,
            base_quote_api,
            cookie_url,
            crumb_url,
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
            #[cfg(feature = "test-mode")]
            api_preference: self
                .api_preference
                .unwrap_or(ApiPreference::ApiThenScrape),
        })
    }
}
