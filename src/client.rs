use crate::error::YfError;
use reqwest::Client;
use reqwest::header::SET_COOKIE;
use url::Url;

#[cfg(feature = "test-mode")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiPreference {
    /// Try the API first, and fall back to scraping on failure (default).
    ApiThenScrape,
    /// Only use the API; fail if it's unavailable.
    ApiOnly,
    /// Only use the HTML scrape method; do not attempt to use the API.
    ScrapeOnly,
}

/// Thin wrapper that holds a configured HTTP client and base URLs.
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

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

impl YfClient {
    pub fn builder() -> YfClientBuilder {
        YfClientBuilder::default()
    }

    pub(crate) async fn ensure_credentials(&mut self) -> Result<(), YfError> {
        if self.crumb.is_some() {
            return Ok(());
        }

        self.get_cookie().await?;

        self.get_crumb_internal().await?;

        Ok(())
    }

    #[cfg(feature = "test-mode")]
    pub(crate) fn api_preference(&self) -> ApiPreference {
        self.api_preference
    }

    async fn get_cookie(&mut self) -> Result<(), YfError> {
        let resp = self.http.get(self.cookie_url.clone())
             .send()
             .await?;

        let cookie = resp
            .headers()
            .get(SET_COOKIE)
            .ok_or(YfError::Data("No cookie received from fc.yahoo.com".into()))?
            .to_str()
            .map_err(|_| YfError::Data("Invalid cookie header format".into()))?
            .to_string();

        self.cookie = Some(cookie);
        Ok(())
    }

    async fn get_crumb_internal(&mut self) -> Result<(), YfError> {
        let cookie = self
            .cookie
            .as_ref()
            .ok_or(YfError::Data("Cookie is missing, cannot get crumb".into()))?;

        // Manually build a client with the specific cookie for this request
        let jar = reqwest::cookie::Jar::default();
        let url = self.crumb_url.clone();
        jar.add_cookie_str(cookie, &url);

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .cookie_provider(std::sync::Arc::new(jar))
            .user_agent(USER_AGENT)
            .build()?;

        let resp = client.get(url).send().await?;

        let crumb = resp.text().await?;
        if crumb.is_empty() || crumb.contains('{') || crumb.contains('<') {
            return Err(YfError::Data(format!("Received invalid crumb: {}", crumb)));
        }

        self.crumb = Some(crumb);
        Ok(())
    }

    pub(crate) fn clear_crumb(&mut self) {
        self.crumb = None;
    }

    pub(crate) fn crumb(&self) -> Option<&str> {
        self.crumb.as_deref()
    }

    pub(crate) fn cookie(&self) -> Option<&str> {
        self.cookie.as_deref()
    }

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
}

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
    /// Override the User-Agent (helpful if Yahoo throttles generic UAs).
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    #[cfg(feature = "test-mode")]
    pub fn api_preference(mut self, pref: ApiPreference) -> Self {
        self.api_preference = Some(pref);
        self
    }

    #[cfg(feature = "test-mode")]
    pub fn preauth(mut self, cookie: impl Into<String>, crumb: impl Into<String>) -> Self {
        self.preauth_cookie = Some(cookie.into());
        self.preauth_crumb = Some(crumb.into());
        self
    }
    
    pub fn base_quote(mut self, url: Url) -> Self {
        self.base_quote = Some(url);
        self
    }

    /// For tests or advanced users: customize the chart base URL.
    pub fn base_chart(mut self, url: Url) -> Self {
        self.base_chart = Some(url);
        self
    }

    pub fn base_quote_api(mut self, url: Url) -> Self {
        self.base_quote_api = Some(url);
        self
    }

    /// Where to fetch the initial cookie from. Defaults to `https://fc.yahoo.com/`.
    pub fn cookie_url(mut self, url: Url) -> Self {
        self.cookie_url = Some(url);
        self
    }
    /// Where to fetch the crumb from. Defaults to `https://query1.finance.yahoo.com/v1/test/getcrumb`.
    pub fn crumb_url(mut self, url: Url) -> Self {
        self.crumb_url = Some(url);
        self
    }

    pub fn build(self) -> Result<YfClient, YfError> {
        let base_chart = self.base_chart.unwrap_or(Url::parse(
            "https://query1.finance.yahoo.com/v8/finance/chart/",
        )?);

        let base_quote = self
            .base_quote
            .unwrap_or(Url::parse("https://finance.yahoo.com/quote/")?);

        let base_quote_api = self.base_quote_api.unwrap_or(Url::parse(
            "https://query2.finance.yahoo.com/v10/finance/quoteSummary/",
        )?);

        let cookie_url = self
            .cookie_url
            .unwrap_or(Url::parse("https://fc.yahoo.com/")?);
        let crumb_url = self
            .crumb_url
            .unwrap_or(Url::parse("https://query1.finance.yahoo.com/v1/test/getcrumb")?);
 
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
            api_preference: self.api_preference.unwrap_or(ApiPreference::ApiThenScrape),
        })
    }
    
}
