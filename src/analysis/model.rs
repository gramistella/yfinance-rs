use serde::Serialize;

/// Counts per period, aligned to Yahoo's recommendationTrend.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecommendationRow {
    pub period: String,            // e.g. "2025-07"
    pub strong_buy: u32,           // strongBuy
    pub buy: u32,                  // buy
    pub hold: u32,                 // hold
    pub sell: u32,                 // sell
    pub strong_sell: u32,          // strongSell
}

/// A compact summary akin to yfinance.recommendations_summary().
/// We expose the most recent period's counts, plus Yahoo's recommendationMean & recommendationKey if present.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecommendationSummary {
    pub latest_period: Option<String>,
    pub strong_buy: u32,
    pub buy: u32,
    pub hold: u32,
    pub sell: u32,
    pub strong_sell: u32,
    pub mean: Option<f64>,            // recommendationMean.raw
    pub mean_key: Option<String>,     // recommendationKey, e.g. "buy", "hold"
}

/// Analyst action (upgrade/downgrade/maintain/etc) history.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpgradeDowngradeRow {
    pub ts: i64,                      // epochGradeDate (seconds)
    pub firm: Option<String>,         // broker/firm
    pub from_grade: Option<String>,   // e.g. "Neutral"
    pub to_grade: Option<String>,     // e.g. "Buy"
    pub action: Option<String>,       // e.g. "upgraded", "downgraded", "main", etc.
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PriceTarget {
    pub mean: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub number_of_analysts: Option<u32>,
}
