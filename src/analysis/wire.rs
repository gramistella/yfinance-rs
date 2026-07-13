use serde::Deserialize;

use crate::core::wire::{BorrowedWireValue, RawDecimal, RawNum, RawNumU64, WireValue};

/* ---------------- Serde mapping (only what we need) ---------------- */

#[derive(Deserialize)]
pub struct V10Result<'a> {
    #[serde(rename = "recommendationTrend", default, borrow)]
    pub(crate) recommendation_trend: BorrowedWireValue<'a, RecommendationTrendNode<'a>>,

    #[serde(rename = "upgradeDowngradeHistory", default, borrow)]
    pub(crate) upgrade_downgrade_history: BorrowedWireValue<'a, UpgradeDowngradeHistoryNode<'a>>,

    #[serde(rename = "financialData", default, borrow)]
    pub(crate) financial_data: BorrowedWireValue<'a, FinancialDataNode>,

    #[serde(rename = "earningsTrend", default, borrow)]
    pub(crate) earnings_trend: BorrowedWireValue<'a, EarningsTrendNode<'a>>,
}

/* --- recommendation trend --- */

#[derive(Deserialize)]
pub struct RecommendationTrendNode<'a> {
    #[serde(default, borrow)]
    pub(crate) trend: BorrowedWireValue<'a, Vec<RecommendationNode>>,
}

#[derive(Deserialize)]
pub struct RecommendationNode {
    #[serde(default)]
    pub(crate) period: WireValue<String>,

    #[serde(rename = "strongBuy")]
    #[serde(default)]
    pub(crate) strong_buy: WireValue<i64>,
    #[serde(default)]
    pub(crate) buy: WireValue<i64>,
    #[serde(default)]
    pub(crate) hold: WireValue<i64>,
    #[serde(default)]
    pub(crate) sell: WireValue<i64>,

    #[serde(rename = "strongSell")]
    #[serde(default)]
    pub(crate) strong_sell: WireValue<i64>,
}

/* --- upgrades / downgrades --- */

#[derive(Deserialize)]
pub struct UpgradeDowngradeHistoryNode<'a> {
    #[serde(default, borrow)]
    pub(crate) history: BorrowedWireValue<'a, Vec<UpgradeNode>>,
}

#[derive(Deserialize)]
pub struct UpgradeNode {
    #[serde(rename = "epochGradeDate")]
    #[serde(default)]
    pub(crate) epoch_grade_date: WireValue<i64>,

    #[serde(default)]
    pub(crate) firm: WireValue<String>,

    #[serde(rename = "toGrade")]
    #[serde(default)]
    pub(crate) to_grade: WireValue<String>,

    #[serde(rename = "fromGrade")]
    #[serde(default)]
    pub(crate) from_grade: WireValue<String>,

    #[serde(default)]
    pub(crate) action: WireValue<String>,
    #[serde(rename = "gradeChange")]
    #[serde(default)]
    pub(crate) grade_change: WireValue<String>,
}

/* --- financial data (price targets) --- */

#[derive(Deserialize)]
pub struct FinancialDataNode {
    #[serde(rename = "financialCurrency")]
    #[serde(default)]
    pub(crate) financial_currency: WireValue<String>,
    #[serde(rename = "targetMeanPrice")]
    #[serde(default)]
    pub(crate) target_mean_price: WireValue<RawDecimal>,
    #[serde(rename = "targetHighPrice")]
    #[serde(default)]
    pub(crate) target_high_price: WireValue<RawDecimal>,
    #[serde(rename = "targetLowPrice")]
    #[serde(default)]
    pub(crate) target_low_price: WireValue<RawDecimal>,
    #[serde(rename = "numberOfAnalystOpinions")]
    #[serde(default)]
    pub(crate) number_of_analyst_opinions: WireValue<RawNumU64>,
    #[serde(rename = "recommendationMean")]
    #[serde(default)]
    pub(crate) recommendation_mean: WireValue<RawDecimal>,
    #[serde(rename = "recommendationKey")]
    #[serde(default)]
    pub(crate) recommendation_key: WireValue<String>,
}

#[derive(Deserialize)]
pub struct EarningsTrendNode<'a> {
    #[serde(default, borrow)]
    pub(crate) trend: BorrowedWireValue<'a, Vec<EarningsTrendItemNode<'a>>>,
}

#[derive(Deserialize)]
pub struct EarningsTrendItemNode<'a> {
    #[serde(default)]
    pub(crate) period: WireValue<String>,
    #[serde(default)]
    pub(crate) growth: WireValue<RawDecimal>,
    #[serde(rename = "earningsEstimate", default, borrow)]
    pub(crate) earnings_estimate: BorrowedWireValue<'a, EarningsEstimateNode>,
    #[serde(rename = "revenueEstimate", default, borrow)]
    pub(crate) revenue_estimate: BorrowedWireValue<'a, RevenueEstimateNode>,
    #[serde(rename = "epsTrend", default, borrow)]
    pub(crate) eps_trend: BorrowedWireValue<'a, EpsTrendNode>,
    #[serde(rename = "epsRevisions", default, borrow)]
    pub(crate) eps_revisions: BorrowedWireValue<'a, EpsRevisionsNode>,
}

#[derive(Deserialize)]
pub struct EarningsEstimateNode {
    #[serde(rename = "earningsCurrency")]
    #[serde(default)]
    pub(crate) earnings_currency: WireValue<String>,
    #[serde(default)]
    pub(crate) avg: WireValue<RawDecimal>,
    #[serde(default)]
    pub(crate) low: WireValue<RawDecimal>,
    #[serde(default)]
    pub(crate) high: WireValue<RawDecimal>,
    #[serde(rename = "yearAgoEps")]
    #[serde(default)]
    pub(crate) year_ago_eps: WireValue<RawDecimal>,
    #[serde(rename = "numberOfAnalysts")]
    #[serde(default)]
    pub(crate) num_analysts: WireValue<RawNumU64>,
    #[serde(default)]
    pub(crate) growth: WireValue<RawDecimal>,
}

#[derive(Deserialize)]
pub struct RevenueEstimateNode {
    #[serde(rename = "revenueCurrency")]
    #[serde(default)]
    pub(crate) revenue_currency: WireValue<String>,
    #[serde(default)]
    pub(crate) avg: WireValue<RawNum<i64>>,
    #[serde(default)]
    pub(crate) low: WireValue<RawNum<i64>>,
    #[serde(default)]
    pub(crate) high: WireValue<RawNum<i64>>,
    #[serde(rename = "yearAgoRevenue")]
    #[serde(default)]
    pub(crate) year_ago_revenue: WireValue<RawNum<i64>>,
    #[serde(rename = "numberOfAnalysts")]
    #[serde(default)]
    pub(crate) num_analysts: WireValue<RawNumU64>,
    #[serde(default)]
    pub(crate) growth: WireValue<RawDecimal>,
}

#[derive(Deserialize)]
pub struct EpsTrendNode {
    #[serde(rename = "epsTrendCurrency")]
    #[serde(default)]
    pub(crate) eps_trend_currency: WireValue<String>,
    #[serde(default)]
    pub(crate) current: WireValue<RawDecimal>,
    #[serde(rename = "7daysAgo")]
    #[serde(default)]
    pub(crate) seven_days_ago: WireValue<RawDecimal>,
    #[serde(rename = "30daysAgo")]
    #[serde(default)]
    pub(crate) thirty_days_ago: WireValue<RawDecimal>,
    #[serde(rename = "60daysAgo")]
    #[serde(default)]
    pub(crate) sixty_days_ago: WireValue<RawDecimal>,
    #[serde(rename = "90daysAgo")]
    #[serde(default)]
    pub(crate) ninety_days_ago: WireValue<RawDecimal>,
}

#[derive(Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct EpsRevisionsNode {
    #[serde(rename = "upLast7days")]
    #[serde(default)]
    pub(crate) up_last_7_days: WireValue<RawNumU64>,
    #[serde(rename = "upLast30days")]
    #[serde(default)]
    pub(crate) up_last_30_days: WireValue<RawNumU64>,
    #[serde(rename = "downLast7days", alias = "downLast7Days")]
    #[serde(default)]
    pub(crate) down_last_7_days: WireValue<RawNumU64>,
    #[serde(rename = "downLast30days", alias = "downLast30Days")]
    #[serde(default)]
    pub(crate) down_last_30_days: WireValue<RawNumU64>,
}
