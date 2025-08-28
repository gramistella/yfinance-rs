use serde::Deserialize;

use crate::core::wire::RawNumI64;

/* ---------------- Serde mapping (only what we need) ---------------- */

#[derive(Deserialize)]
pub(crate) struct V10Result {
    #[serde(rename = "recommendationTrend")]
    pub(crate) recommendation_trend: Option<RecommendationTrendNode>,

    #[serde(rename = "recommendationMean")]
    pub(crate) recommendation_mean: Option<RecommendationMeanNode>,

    #[serde(rename = "upgradeDowngradeHistory")]
    pub(crate) upgrade_downgrade_history: Option<UpgradeDowngradeHistoryNode>,

    #[serde(rename = "financialData")]
    pub(crate) financial_data: Option<FinancialDataNode>,

    #[serde(rename = "earningsTrend")]
    pub(crate) earnings_trend: Option<EarningsTrendNode>,
}

/* --- recommendation trend --- */

#[derive(Deserialize)]
pub(crate) struct RecommendationTrendNode {
    pub(crate) trend: Option<Vec<RecommendationNode>>,
}

#[derive(Deserialize)]
pub(crate) struct RecommendationNode {
    pub(crate) period: Option<String>,

    #[serde(rename = "strongBuy")]
    pub(crate) strong_buy: Option<i64>,
    pub(crate) buy: Option<i64>,
    pub(crate) hold: Option<i64>,
    pub(crate) sell: Option<i64>,

    #[serde(rename = "strongSell")]
    pub(crate) strong_sell: Option<i64>,
}

/* --- recommendation mean / key --- */

#[derive(Deserialize)]
pub(crate) struct RecommendationMeanNode {
    #[serde(rename = "recommendationMean")]
    pub(crate) recommendation_mean: Option<RawNum>,

    #[serde(rename = "recommendationKey")]
    pub(crate) recommendation_key: Option<String>,
}

/* --- upgrades / downgrades --- */

#[derive(Deserialize)]
pub(crate) struct UpgradeDowngradeHistoryNode {
    pub(crate) history: Option<Vec<UpgradeNode>>,
}

#[derive(Deserialize)]
pub(crate) struct UpgradeNode {
    #[serde(rename = "epochGradeDate")]
    pub(crate) epoch_grade_date: Option<i64>,

    pub(crate) firm: Option<String>,

    #[serde(rename = "toGrade")]
    pub(crate) to_grade: Option<String>,

    #[serde(rename = "fromGrade")]
    pub(crate) from_grade: Option<String>,

    pub(crate) action: Option<String>,
    #[serde(rename = "gradeChange")]
    pub(crate) grade_change: Option<String>,
}

/* --- financial data (price targets) --- */

#[derive(Deserialize)]
pub(crate) struct FinancialDataNode {
    #[serde(rename = "targetMeanPrice")]
    pub(crate) target_mean_price: Option<RawNum>,
    #[serde(rename = "targetHighPrice")]
    pub(crate) target_high_price: Option<RawNum>,
    #[serde(rename = "targetLowPrice")]
    pub(crate) target_low_price: Option<RawNum>,
    #[serde(rename = "numberOfAnalystOpinions")]
    pub(crate) number_of_analyst_opinions: Option<RawNum>,
}

#[derive(Deserialize)]
pub(crate) struct EarningsTrendNode {
    pub(crate) trend: Option<Vec<EarningsTrendItemNode>>,
}

#[derive(Deserialize)]
pub(crate) struct EarningsTrendItemNode {
    pub(crate) period: Option<String>,
    pub(crate) growth: Option<RawNum>,
    #[serde(rename = "earningsEstimate")]
    pub(crate) earnings_estimate: Option<EarningsEstimateNode>,
    #[serde(rename = "revenueEstimate")]
    pub(crate) revenue_estimate: Option<RevenueEstimateNode>,
    #[serde(rename = "epsTrend")]
    pub(crate) eps_trend: Option<EpsTrendNode>,
    #[serde(rename = "epsRevisions")]
    pub(crate) eps_revisions: Option<EpsRevisionsNode>,
}

#[derive(Deserialize)]
pub(crate) struct EarningsEstimateNode {
    pub(crate) avg: Option<RawNum>,
    pub(crate) low: Option<RawNum>,
    pub(crate) high: Option<RawNum>,
    #[serde(rename = "yearAgoEps")]
    pub(crate) year_ago_eps: Option<RawNum>,
    #[serde(rename = "numberOfAnalysts")]
    pub(crate) num_analysts: Option<RawNum>,
    pub(crate) growth: Option<RawNum>,
}

#[derive(Deserialize)]
pub(crate) struct RevenueEstimateNode {
    pub(crate) avg: Option<RawNumI64>,
    pub(crate) low: Option<RawNumI64>,
    pub(crate) high: Option<RawNumI64>,
    #[serde(rename = "yearAgoRevenue")]
    pub(crate) year_ago_revenue: Option<RawNumI64>,
    #[serde(rename = "numberOfAnalysts")]
    pub(crate) num_analysts: Option<RawNum>,
    pub(crate) growth: Option<RawNum>,
}

#[derive(Deserialize)]
pub(crate) struct EpsTrendNode {
    pub(crate) current: Option<RawNum>,
    #[serde(rename = "7daysAgo")]
    pub(crate) seven_days_ago: Option<RawNum>,
    #[serde(rename = "30daysAgo")]
    pub(crate) thirty_days_ago: Option<RawNum>,
    #[serde(rename = "60daysAgo")]
    pub(crate) sixty_days_ago: Option<RawNum>,
    #[serde(rename = "90daysAgo")]
    pub(crate) ninety_days_ago: Option<RawNum>,
}

#[derive(Deserialize)]
pub(crate) struct EpsRevisionsNode {
    #[serde(rename = "upLast7days")]
    pub(crate) up_last_7_days: Option<RawNum>,
    #[serde(rename = "upLast30days")]
    pub(crate) up_last_30_days: Option<RawNum>,
    #[serde(rename = "downLast7days")]
    pub(crate) down_last_7_days: Option<RawNum>,
    #[serde(rename = "downLast30days")]
    pub(crate) down_last_30_days: Option<RawNum>,
}

/* --- shared small wrappers --- */

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct RawNum {
    pub(crate) raw: Option<f64>,
}
