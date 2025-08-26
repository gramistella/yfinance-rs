use crate::core::{
    client::{CacheMode, RetryConfig},
    quotesummary, YfClient, YfError,
};
use super::model::{
    InstitutionalHolder, InsiderRosterHolder, InsiderTransaction, MajorHolder,
    NetSharePurchaseActivity,
};
use super::wire::{RawNum, V10Result};

const MODULES: &str = "institutionOwnership,fundOwnership,majorHoldersBreakdown,insiderTransactions,insiderHolders,netSharePurchaseActivity";

async fn fetch_holders_modules(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<V10Result, YfError> {
    quotesummary::fetch_module_result(client, symbol, MODULES, "holders", cache_mode, retry_override).await
}

fn f<T: Copy>(r: Option<RawNum<T>>) -> Option<T> {
    r.and_then(|n| n.raw)
}

pub(super) async fn major_holders(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<MajorHolder>, YfError> {
    let root = fetch_holders_modules(client, symbol, cache_mode, retry_override).await?;
    let breakdown = root
        .major_holders_breakdown
        .ok_or_else(|| YfError::Data("majorHoldersBreakdown missing".into()))?;

    let mut result = Vec::new();
    let percent_fmt = |v: Option<f64>| v.map(|p| format!("{:.2}%", p * 100.0)).unwrap_or_default();

    if let Some(v) = f(breakdown.insiders_percent_held) {
        result.push(MajorHolder {
            category: "% of Shares Held by All Insider".into(),
            value: percent_fmt(Some(v)),
        });
    }
    if let Some(v) = f(breakdown.institutions_percent_held) {
        result.push(MajorHolder {
            category: "% of Shares Held by Institutions".into(),
            value: percent_fmt(Some(v)),
        });
    }
    if let Some(v) = f(breakdown.institutions_float_percent_held) {
        result.push(MajorHolder {
            category: "% of Float Held by Institutions".into(),
            value: percent_fmt(Some(v)),
        });
    }
    if let Some(v) = f(breakdown.institutions_count) {
        result.push(MajorHolder {
            category: "Number of Institutions Holding Shares".into(),
            value: v.to_string(),
        });
    }

    Ok(result)
}

fn map_ownership_list(node: Option<super::wire::OwnershipNode>) -> Vec<InstitutionalHolder> {
    node.and_then(|n| n.ownership_list)
        .unwrap_or_default()
        .into_iter()
        .map(|h| InstitutionalHolder {
            holder: h.organization.unwrap_or_default(),
            shares: f(h.shares).unwrap_or(0),
            date_reported: h.date_reported.and_then(|d| d.raw).unwrap_or(0),
            pct_held: f(h.pct_held).unwrap_or(0.0),
            value: f(h.value).unwrap_or(0),
        })
        .collect()
}

pub(super) async fn institutional_holders(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<InstitutionalHolder>, YfError> {
    let root = fetch_holders_modules(client, symbol, cache_mode, retry_override).await?;
    Ok(map_ownership_list(root.institution_ownership))
}

pub(super) async fn mutual_fund_holders(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<InstitutionalHolder>, YfError> {
    let root = fetch_holders_modules(client, symbol, cache_mode, retry_override).await?;
    Ok(map_ownership_list(root.fund_ownership))
}

pub(super) async fn insider_transactions(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<InsiderTransaction>, YfError> {
    let root = fetch_holders_modules(client, symbol, cache_mode, retry_override).await?;
    let transactions = root
        .insider_transactions
        .and_then(|it| it.transactions)
        .unwrap_or_default();

    Ok(transactions
        .into_iter()
        .map(|t| InsiderTransaction {
            insider: t.insider.unwrap_or_default(),
            position: t.position.unwrap_or_default(),
            transaction: t.transaction.unwrap_or_default(),
            shares: f(t.shares).unwrap_or(0),
            value: f(t.value).unwrap_or(0),
            start_date: t.start_date.and_then(|d| d.raw).unwrap_or(0),
            url: t.url.unwrap_or_default(),
        })
        .collect())
}

pub(super) async fn insider_roster_holders(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<InsiderRosterHolder>, YfError> {
    let root = fetch_holders_modules(client, symbol, cache_mode, retry_override).await?;
    let holders = root
        .insider_holders
        .and_then(|ih| ih.holders)
        .unwrap_or_default();

    Ok(holders
        .into_iter()
        .map(|h| InsiderRosterHolder {
            name: h.name.unwrap_or_default(),
            position: h.relation.unwrap_or_default(),
            most_recent_transaction: h.most_recent_transaction.unwrap_or_default(),
            latest_transaction_date: h.latest_transaction_date.and_then(|d| d.raw).unwrap_or(0),
            shares_owned_directly: f(h.shares_owned_directly).unwrap_or(0),
            position_direct_date: h.position_direct_date.and_then(|d| d.raw).unwrap_or(0),
        })
        .collect())
}

pub(super) async fn net_share_purchase_activity(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Option<NetSharePurchaseActivity>, YfError> {
    let root = fetch_holders_modules(client, symbol, cache_mode, retry_override).await?;
    Ok(root.net_share_purchase_activity.map(|n| {
        NetSharePurchaseActivity {
            period: n.period.unwrap_or_default(),
            buy_info_shares: f(n.buy_info_shares).unwrap_or(0),
            buy_info_count: f(n.buy_info_count).unwrap_or(0),
            sell_info_shares: f(n.sell_info_shares).unwrap_or(0),
            sell_info_count: f(n.sell_info_count).unwrap_or(0),
            net_info_shares: f(n.net_info_shares).unwrap_or(0),
            net_info_count: f(n.net_info_count).unwrap_or(0),
            total_insider_shares: f(n.total_insider_shares).unwrap_or(0),
            net_percent_insider_shares: f(n.net_percent_insider_shares).unwrap_or(0.0),
        }
    }))
}