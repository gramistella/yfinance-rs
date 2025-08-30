use serde::Serialize;

/// Represents a single news article for a ticker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewsArticle {
    /// A unique identifier for the article.
    pub uuid: String,
    /// The headline of the article.
    pub title: String,
    /// The publisher of the article (e.g., "Reuters", "Associated Press").
    pub publisher: Option<String>,
    /// A direct link to the article.
    pub link: Option<String>,
    /// The Unix timestamp (in seconds) of when the article was published.
    pub provider_publish_time: i64,
}
