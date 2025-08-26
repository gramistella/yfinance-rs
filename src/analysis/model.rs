use serde::Serialize;

/// A row representing analyst recommendation counts for a specific period.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecommendationRow {
    /// The period for the recommendation counts (e.g., "0m", "-1m").
    pub period: String,
    /// The number of "Strong Buy" recommendations.
    pub strong_buy: u32,
    /// The number of "Buy" recommendations.
    pub buy: u32,
    /// The number of "Hold" recommendations.
    pub hold: u32,
    /// The number of "Sell" recommendations.
    pub sell: u32,
    /// The number of "Strong Sell" recommendations.
    pub strong_sell: u32,
}

/// A compact summary of the latest analyst recommendations.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecommendationSummary {
    /// The most recent period for which recommendations are available (e.g., "0m").
    pub latest_period: Option<String>,
    /// The number of "Strong Buy" recommendations in the latest period.
    pub strong_buy: u32,
    /// The number of "Buy" recommendations in the latest period.
    pub buy: u32,
    /// The number of "Hold" recommendations in the latest period.
    pub hold: u32,
    /// The number of "Sell" recommendations in the latest period.
    pub sell: u32,
    /// The number of "Strong Sell" recommendations in the latest period.
    pub strong_sell: u32,
    /// The mean recommendation score (e.g., 1.0 for Strong Buy, 5.0 for Strong Sell).
    pub mean: Option<f64>,
    /// The categorical key for the mean recommendation (e.g., "buy", "hold").
    pub mean_key: Option<String>,
}

/// A row representing a single analyst upgrade or downgrade action.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpgradeDowngradeRow {
    /// The Unix timestamp (in seconds) of the action.
    pub ts: i64,
    /// The name of the firm or analyst making the recommendation.
    pub firm: Option<String>,
    /// The previous rating or grade.
    pub from_grade: Option<String>,
    /// The new rating or grade.
    pub to_grade: Option<String>,
    /// The type of action taken (e.g., "main", "upgraded", "downgraded").
    pub action: Option<String>,
}

/// Analyst price target summary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PriceTarget {
    /// The mean analyst price target.
    pub mean: Option<f64>,
    /// The highest analyst price target.
    pub high: Option<f64>,
    /// The lowest analyst price target.
    pub low: Option<f64>,
    /// The number of analysts providing an opinion.
    pub number_of_analysts: Option<u32>,
}
