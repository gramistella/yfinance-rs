use serde::Serialize;

/// A row representing analyst recommendation counts for a specific period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

/// Represents a single row of earnings trend data for a specific period.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EarningsTrendRow {
    /// The period the trend data applies to (e.g., "0q", "+1q", "0y", "+1y").
    pub period: String,
    /// The growth rate.
    pub growth: Option<f64>,
    /// Average earnings estimate.
    pub earnings_estimate_avg: Option<f64>,
    /// Low earnings estimate.
    pub earnings_estimate_low: Option<f64>,
    /// High earnings estimate.
    pub earnings_estimate_high: Option<f64>,
    /// Earnings per share from a year ago.
    pub earnings_estimate_year_ago_eps: Option<f64>,
    /// Number of analysts providing earnings estimates.
    pub earnings_estimate_num_analysts: Option<u32>,
    /// Estimated earnings growth.
    pub earnings_estimate_growth: Option<f64>,
    /// Average revenue estimate.
    pub revenue_estimate_avg: Option<i64>,
    /// Low revenue estimate.
    pub revenue_estimate_low: Option<i64>,
    /// High revenue estimate.
    pub revenue_estimate_high: Option<i64>,
    /// Revenue from a year ago.
    pub revenue_estimate_year_ago_revenue: Option<i64>,
    /// Number of analysts providing revenue estimates.
    pub revenue_estimate_num_analysts: Option<u32>,
    /// Estimated revenue growth.
    pub revenue_estimate_growth: Option<f64>,
    /// Current EPS trend.
    pub eps_trend_current: Option<f64>,
    /// EPS trend from 7 days ago.
    pub eps_trend_7_days_ago: Option<f64>,
    /// EPS trend from 30 days ago.
    pub eps_trend_30_days_ago: Option<f64>,
    /// EPS trend from 60 days ago.
    pub eps_trend_60_days_ago: Option<f64>,
    /// EPS trend from 90 days ago.
    pub eps_trend_90_days_ago: Option<f64>,
    /// Number of upward EPS revisions in the last 7 days.
    pub eps_revisions_up_last_7_days: Option<u32>,
    /// Number of upward EPS revisions in the last 30 days.
    pub eps_revisions_up_last_30_days: Option<u32>,
    /// Number of downward EPS revisions in the last 7 days.
    pub eps_revisions_down_last_7_days: Option<u32>,
    /// Number of downward EPS revisions in the last 30 days.
    pub eps_revisions_down_last_30_days: Option<u32>,
}
