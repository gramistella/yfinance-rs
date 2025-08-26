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
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            NewsTab::News => "latestNews",
            NewsTab::All => "newsAll",
            NewsTab::PressReleases => "pressRelease",
        }
    }
}

/// A builder for fetching news articles for a specific symbol.
pub struct NewsBuilder<'a> {
    client: &'a YfClient,
    symbol: String,
    count: u32,
    tab: NewsTab,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl<'a> NewsBuilder<'a> {
    /// Creates a new `NewsBuilder` for a given symbol.
    pub fn new(client: &'a YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client,
            symbol: symbol.into(),
            count: 10,
            tab: NewsTab::default(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for this specific API call.
    /// Note: Caching is not currently implemented for news requests.
    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call.
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Sets the maximum number of news articles to return.
    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// Sets the category of news to fetch.
    pub fn tab(mut self, tab: NewsTab) -> Self {
        self.tab = tab;
        self
    }

    /// Executes the request and fetches the news articles.
    pub async fn fetch(self) -> Result<Vec<NewsArticle>, YfError> {
        api::fetch_news(
            self.client,
            &self.symbol,
            self.count,
            self.tab,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }
}
