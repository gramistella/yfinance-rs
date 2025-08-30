mod api;
mod model;
mod wire;

pub use model::NewsArticle;

use crate::{
    YfClient, YfError,
    core::client::{CacheMode, RetryConfig},
};

/// The category of news to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewsTab {
    /// The latest news articles. (Default)
    #[default]
    News,
    /// All news-related content.
    All,
    /// Official press releases.
    PressReleases,
}

impl NewsTab {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::News => "latestNews",
            Self::All => "newsAll",
            Self::PressReleases => "pressRelease",
        }
    }
}

/// A builder for fetching news articles for a specific symbol.
pub struct NewsBuilder {
    client: YfClient,
    symbol: String,
    count: u32,
    tab: NewsTab,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl NewsBuilder {
    /// Creates a new `NewsBuilder` for a given symbol.
    pub fn new(client: &YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client: client.clone(),
            symbol: symbol.into(),
            count: 10,
            tab: NewsTab::default(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for this specific API call.
    /// Note: Caching is not currently implemented for news requests.
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

    /// Sets the maximum number of news articles to return.
    #[must_use]
    pub const fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// Sets the category of news to fetch.
    #[must_use]
    pub const fn tab(mut self, tab: NewsTab) -> Self {
        self.tab = tab;
        self
    }

    /// Executes the request and fetches the news articles.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the request to the Yahoo Finance API fails,
    /// if the response cannot be parsed, or if there's a network issue.
    pub async fn fetch(self) -> Result<Vec<NewsArticle>, YfError> {
        api::fetch_news(
            &self.client,
            &self.symbol,
            self.count,
            self.tab,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
