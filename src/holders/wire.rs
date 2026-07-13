use crate::core::wire::{RawDate, RawDecimal, RawNum, WireValue};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct V10Result {
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
pub struct OwnershipNode {
    #[serde(rename = "ownershipList")]
    pub(crate) ownership_list: Option<Vec<Value>>,
}

#[derive(Deserialize)]
pub struct InstitutionalHolderNode {
    pub(crate) organization: Option<String>,
    #[serde(rename = "position")]
    #[serde(default)]
    pub(crate) shares: WireValue<RawNum<u64>>,
    #[serde(rename = "reportDate")]
    #[serde(default)]
    pub(crate) date_reported: WireValue<RawDate>,
    #[serde(rename = "pctHeld")]
    #[serde(default)]
    pub(crate) pct_held: WireValue<RawDecimal>,
    #[serde(default)]
    pub(crate) value: WireValue<RawNum<u64>>,
}

#[derive(Deserialize)]
pub struct MajorHoldersBreakdownNode {
    #[serde(rename = "insidersPercentHeld")]
    #[serde(default)]
    pub(crate) insiders: WireValue<RawDecimal>,
    #[serde(rename = "institutionsPercentHeld")]
    #[serde(default)]
    pub(crate) institutions: WireValue<RawDecimal>,
    #[serde(rename = "institutionsFloatPercentHeld")]
    #[serde(default)]
    pub(crate) institutions_float: WireValue<RawDecimal>,
}

#[derive(Deserialize)]
pub struct InsiderTransactionsNode {
    pub(crate) transactions: Option<Vec<Value>>,
}

#[derive(Deserialize)]
pub struct InsiderTransactionNode {
    #[serde(rename = "filerName")]
    pub(crate) insider: Option<String>,
    #[serde(rename = "filerRelation")]
    pub(crate) position: Option<String>,
    #[serde(rename = "transactionText")]
    pub(crate) transaction: Option<String>,
    #[serde(default)]
    pub(crate) shares: WireValue<RawNum<u64>>,
    #[serde(default)]
    pub(crate) value: WireValue<RawNum<u64>>,
    #[serde(rename = "startDate")]
    #[serde(default)]
    pub(crate) start_date: WireValue<RawDate>,
    #[serde(rename = "filerUrl")]
    pub(crate) url: Option<String>,
}

#[derive(Deserialize)]
pub struct InsiderHoldersNode {
    pub(crate) holders: Option<Vec<Value>>,
}

#[derive(Deserialize)]
pub struct InsiderRosterHolderNode {
    pub(crate) name: Option<String>,
    pub(crate) relation: Option<String>,
    #[serde(rename = "transactionDescription")]
    pub(crate) most_recent_transaction: Option<String>,
    #[serde(rename = "latestTransDate")]
    #[serde(default)]
    pub(crate) latest_transaction_date: WireValue<RawDate>,
    #[serde(rename = "positionDirect")]
    #[serde(default)]
    pub(crate) shares_owned_directly: WireValue<RawNum<u64>>,
    #[serde(rename = "positionDirectDate")]
    #[serde(default)]
    pub(crate) position_direct_date: WireValue<RawDate>,
}

#[derive(Deserialize)]
pub struct NetSharePurchaseActivityNode {
    pub(crate) period: Option<String>,
    #[serde(rename = "buyInfoShares")]
    #[serde(default)]
    pub(crate) buy_info_shares: WireValue<RawNum<u64>>,
    #[serde(rename = "buyInfoCount")]
    #[serde(default)]
    pub(crate) buy_info_count: WireValue<RawNum<u64>>,
    #[serde(rename = "sellInfoShares")]
    #[serde(default)]
    pub(crate) sell_info_shares: WireValue<RawNum<u64>>,
    #[serde(rename = "sellInfoCount")]
    #[serde(default)]
    pub(crate) sell_info_count: WireValue<RawNum<u64>>,
    #[serde(rename = "netInfoShares")]
    #[serde(default)]
    pub(crate) net_info_shares: WireValue<RawNum<i64>>,
    #[serde(rename = "netInfoCount")]
    #[serde(default)]
    pub(crate) net_info_count: WireValue<RawNum<i64>>,
    #[serde(rename = "totalInsiderShares")]
    #[serde(default)]
    pub(crate) total_insider_shares: WireValue<RawNum<u64>>,
    #[serde(rename = "netPercentInsiderShares")]
    #[serde(default)]
    pub(crate) net_percent_insider_shares: WireValue<RawDecimal>,
}
