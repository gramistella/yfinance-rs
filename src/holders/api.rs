use super::model::{
    InsiderRosterHolder, InsiderTransaction, InstitutionalHolder, MajorHolder,
    NetSharePurchaseActivity,
};
use super::wire::V10Result;
use crate::core::wire::{from_raw, from_raw_date};
use crate::core::{
    YfClient, YfError,
    client::{CacheMode, RetryConfig},
    conversions::{
        i64_to_datetime,
        string_to_insider_position,
        string_to_transaction_type,
        u64_to_money_with_currency,
    },
    quotesummary,
};
use chrono::DateTime;
use paft::prelude::Currency;

#[inline]
#[allow(clippy::cast_precision_loss)]
const fn u64_to_f64(n: u64) -> f64 { n as f64 }

const MODULES: &str = "institutionOwnership,fundOwnership,majorHoldersBreakdown,insiderTransactions,insiderHolders,netSharePurchaseActivity";

async fn fetch_holders_modules(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<V10Result, YfError> {
    quotesummary::fetch_module_result(
        client,
        symbol,
        MODULES,
        "holders",
        cache_mode,
        retry_override,
    )
    .await
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
        .ok_or_else(|| YfError::MissingData("majorHoldersBreakdown missing".into()))?;

    let mut result = Vec::new();

    if let Some(v) = from_raw(breakdown.insiders_percent_held) {
        result.push(MajorHolder {
            category: "% of Shares Held by All Insiders".into(),
            value: v,
        });
    }
    if let Some(v) = from_raw(breakdown.institutions_percent_held) {
        result.push(MajorHolder {
            category: "% of Shares Held by Institutions".into(),
            value: v,
        });
    }
    if let Some(v) = from_raw(breakdown.institutions_float_percent_held) {
        result.push(MajorHolder {
            category: "% of Float Held by Institutions".into(),
            value: v,
        });
    }
    if let Some(v) = from_raw(breakdown.institutions_count) {
        result.push(MajorHolder {
            category: "Number of Institutions Holding Shares".into(),
            value: u64_to_f64(v),
        });
    }

    Ok(result)
}

fn map_ownership_list(
    node: Option<super::wire::OwnershipNode>,
    currency: &Currency,
) -> Vec<InstitutionalHolder> {
    node.and_then(|n| n.ownership_list)
        .unwrap_or_default()
        .into_iter()
        .map(|h| InstitutionalHolder {
            holder: h.organization.unwrap_or_default(),
            shares: from_raw(h.shares),
            date_reported: from_raw_date(h.date_reported).map_or_else(
                || DateTime::from_timestamp(0, 0).unwrap_or_default(),
                i64_to_datetime,
            ),
            pct_held: from_raw(h.pct_held),
            value: from_raw(h.value)
                .map(|v| u64_to_money_with_currency(v, currency.clone())),
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
    let currency = client.reporting_currency(symbol, None).await;
    Ok(map_ownership_list(root.institution_ownership, &currency))
}

pub(super) async fn mutual_fund_holders(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<InstitutionalHolder>, YfError> {
    let root = fetch_holders_modules(client, symbol, cache_mode, retry_override).await?;
    let currency = client.reporting_currency(symbol, None).await;
    Ok(map_ownership_list(root.fund_ownership, &currency))
}

pub(super) async fn insider_transactions(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<InsiderTransaction>, YfError> {
    let root = fetch_holders_modules(client, symbol, cache_mode, retry_override).await?;
    let currency = client.reporting_currency(symbol, None).await;
    let transactions = root
        .insider_transactions
        .and_then(|it| it.transactions)
        .unwrap_or_default();

    Ok(transactions
        .into_iter()
        .map(|t| InsiderTransaction {
            insider: t.insider.unwrap_or_default(),
            position: string_to_insider_position(t.position.unwrap_or_default()),
            transaction_type: string_to_transaction_type(t.transaction.unwrap_or_default()),
            shares: from_raw(t.shares),
            value: from_raw(t.value)
                .map(|v| u64_to_money_with_currency(v, currency.clone())),
            transaction_date: from_raw_date(t.start_date).map_or_else(
                || DateTime::from_timestamp(0, 0).unwrap_or_default(),
                i64_to_datetime,
            ),
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
            position: string_to_insider_position(h.relation.unwrap_or_default()),
            most_recent_transaction: string_to_transaction_type(
                h.most_recent_transaction.unwrap_or_default(),
            ),
            latest_transaction_date: from_raw_date(h.latest_transaction_date).map_or_else(
                || DateTime::from_timestamp(0, 0).unwrap_or_default(),
                i64_to_datetime,
            ),
            shares_owned_directly: from_raw(h.shares_owned_directly),
            position_direct_date: from_raw_date(h.position_direct_date).map_or_else(
                || DateTime::from_timestamp(0, 0).unwrap_or_default(),
                i64_to_datetime,
            ),
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
    Ok(root
        .net_share_purchase_activity
        .map(|n| NetSharePurchaseActivity {
            period: n.period.unwrap_or_default(),
            buy_shares: from_raw(n.buy_info_shares),
            buy_count: from_raw(n.buy_info_count),
            sell_shares: from_raw(n.sell_info_shares),
            sell_count: from_raw(n.sell_info_count),
            net_shares: from_raw(n.net_info_shares),
            net_count: from_raw(n.net_info_count),
            total_insider_shares: from_raw(n.total_insider_shares),
            net_percent_insider_shares: from_raw(n.net_percent_insider_shares),
        }))
}
