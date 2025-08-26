use serde::Deserialize;

// Wrapper for numbers that come in { "raw": ..., "fmt": ... } format
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct RawNum<T> {
    pub(crate) raw: Option<T>,
    // fmt: Option<String>,
}

// Wrapper for dates that come in { "raw": ..., "fmt": ... } format
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct RawDate {
    pub(crate) raw: Option<i64>,
    // pub(crate) fmt: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct V10Result {
    #[serde(rename = "institutionOwnership")]
    pub(crate) institution_ownership: Option<OwnershipNode>,
    #[serde(rename = "fundOwnership")]
    pub(crate) fund_ownership: Option<OwnershipNode>,
    #[serde(rename = "majorHoldersBreakdown")]
    pub(crate) major_holders_breakdown: Option<MajorHoldersBreakdownNode>,
    #[serde(rename = "insiderTransactions")]
    pub(crate) insider_transactions: Option<InsiderTransactionsNode>,
    #[serde(rename = "insiderHolders")]
    pub(crate) insider_holders: Option<InsiderHoldersNode>,
    #[serde(rename = "netSharePurchaseActivity")]
    pub(crate) net_share_purchase_activity: Option<NetSharePurchaseActivityNode>,
}

#[derive(Deserialize)]
pub(crate) struct OwnershipNode {
    #[serde(rename = "ownershipList")]
    pub(crate) ownership_list: Option<Vec<InstitutionalHolderNode>>,
}

#[derive(Deserialize)]
pub(crate) struct InstitutionalHolderNode {
    pub(crate) organization: Option<String>,
    #[serde(rename = "position")]
    pub(crate) shares: Option<RawNum<u64>>,
    #[serde(rename = "reportDate")]
    pub(crate) date_reported: Option<RawDate>,
    #[serde(rename = "pctHeld")]
    pub(crate) pct_held: Option<RawNum<f64>>,
    pub(crate) value: Option<RawNum<u64>>,
}

#[derive(Deserialize)]
pub(crate) struct MajorHoldersBreakdownNode {
    #[serde(rename = "insidersPercentHeld")]
    pub(crate) insiders_percent_held: Option<RawNum<f64>>,
    #[serde(rename = "institutionsPercentHeld")]
    pub(crate) institutions_percent_held: Option<RawNum<f64>>,
    #[serde(rename = "institutionsFloatPercentHeld")]
    pub(crate) institutions_float_percent_held: Option<RawNum<f64>>,
    #[serde(rename = "institutionsCount")]
    pub(crate) institutions_count: Option<RawNum<u64>>,
}

#[derive(Deserialize)]
pub(crate) struct InsiderTransactionsNode {
    pub(crate) transactions: Option<Vec<InsiderTransactionNode>>,
}

#[derive(Deserialize)]
pub(crate) struct InsiderTransactionNode {
    #[serde(rename = "filerName")]
    pub(crate) insider: Option<String>,
    #[serde(rename = "filerRelation")]
    pub(crate) position: Option<String>,
    #[serde(rename = "transactionText")]
    pub(crate) transaction: Option<String>,
    pub(crate) shares: Option<RawNum<u64>>,
    pub(crate) value: Option<RawNum<u64>>,
    #[serde(rename = "startDate")]
    pub(crate) start_date: Option<RawDate>,
    #[serde(rename = "filerUrl")]
    pub(crate) url: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct InsiderHoldersNode {
    pub(crate) holders: Option<Vec<InsiderRosterHolderNode>>,
}

#[derive(Deserialize)]
pub(crate) struct InsiderRosterHolderNode {
    pub(crate) name: Option<String>,
    pub(crate) relation: Option<String>,
    #[serde(rename = "transactionDescription")]
    pub(crate) most_recent_transaction: Option<String>,
    #[serde(rename = "latestTransDate")]
    pub(crate) latest_transaction_date: Option<RawDate>,
    #[serde(rename = "positionDirect")]
    pub(crate) shares_owned_directly: Option<RawNum<u64>>,
    #[serde(rename = "positionDirectDate")]
    pub(crate) position_direct_date: Option<RawDate>,
}

#[derive(Deserialize)]
pub(crate) struct NetSharePurchaseActivityNode {
    pub(crate) period: Option<String>,
    #[serde(rename = "buyInfoShares")]
    pub(crate) buy_info_shares: Option<RawNum<u64>>,
    #[serde(rename = "buyInfoCount")]
    pub(crate) buy_info_count: Option<RawNum<u64>>,
    #[serde(rename = "sellInfoShares")]
    pub(crate) sell_info_shares: Option<RawNum<u64>>,
    #[serde(rename = "sellInfoCount")]
    pub(crate) sell_info_count: Option<RawNum<u64>>,
    #[serde(rename = "netInfoShares")]
    pub(crate) net_info_shares: Option<RawNum<i64>>,
    #[serde(rename = "netInfoCount")]
    pub(crate) net_info_count: Option<RawNum<i64>>,
    #[serde(rename = "totalInsiderShares")]
    pub(crate) total_insider_shares: Option<RawNum<u64>>,
    #[serde(rename = "netPercentInsiderShares")]
    pub(crate) net_percent_insider_shares: Option<RawNum<f64>>,
}