use serde::Deserialize;

/* ---------------- Serde mapping (only what we need) ---------------- */

#[derive(Deserialize)]
pub(crate) struct V10Envelope {
    #[serde(rename = "quoteSummary")]
    pub(crate) quote_summary: Option<V10QuoteSummary>,
}

#[derive(Deserialize)]
pub(crate) struct V10QuoteSummary {
    pub(crate) result: Option<Vec<V10Result>>,
    pub(crate) error: Option<V10Error>,
}

#[derive(Deserialize)]
pub(crate) struct V10Error {
    pub(crate) description: String,
}

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

/* --- shared small wrappers --- */

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct RawNum {
    pub(crate) raw: Option<f64>,
}
