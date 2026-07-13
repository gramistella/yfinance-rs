use super::model::{
    InsiderRosterHolder, InsiderTransaction, InstitutionalHolder, MajorHolder,
    NetSharePurchaseActivity,
};
use super::wire::{
    InsiderRosterHolderNode, InsiderTransactionNode, InstitutionalHolderNode, OwnershipNode,
    V10Result,
};
use crate::core::{
    CallOptions, ProjectionContext, ProjectionIssue, YfClient, YfError, YfResponse,
    conversions::{string_to_insider_position, string_to_transaction_type},
    currency_resolver::{
        CurrencyPurpose, ResolvedCurrencyUnit, TradingCurrencyEvidence, project_currency_resolution,
    },
    diagnostics::{
        WireProjection, nonempty_string, optional_money_u64_with_currency_issue,
        optional_projected, required_nonempty_string, required_parsed, required_wire_date,
    },
    quotesummary,
};
use paft::fundamentals::holders::{InsiderPosition, TransactionType};
use paft::{Decimal, Ratio};

const INSTITUTION_OWNERSHIP_MODULE: &str = "institutionOwnership";
const FUND_OWNERSHIP_MODULE: &str = "fundOwnership";
const MAJOR_HOLDERS_MODULE: &str = "majorHoldersBreakdown";
const INSIDER_TRANSACTIONS_MODULE: &str = "insiderTransactions";
const INSIDER_HOLDERS_MODULE: &str = "insiderHolders";
const NET_SHARE_PURCHASE_ACTIVITY_MODULE: &str = "netSharePurchaseActivity";
const INSTITUTION_OWNERSHIP: OwnershipFeatureNames = OwnershipFeatureNames {
    module: "institutionOwnership",
    list: "institutionOwnership.ownershipList",
};
const FUND_OWNERSHIP: OwnershipFeatureNames = OwnershipFeatureNames {
    module: "fundOwnership",
    list: "fundOwnership.ownershipList",
};

#[derive(Clone, Copy)]
struct OwnershipFeatureNames {
    module: &'static str,
    list: &'static str,
}

async fn fetch_holders_modules(
    client: &YfClient,
    symbol: &str,
    modules: &str,
    options: &CallOptions,
) -> Result<V10Result, YfError> {
    quotesummary::fetch_module_result(client, symbol, modules, "holders", options).await
}

pub(super) async fn major_holders(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<YfResponse<Vec<MajorHolder>>, YfError> {
    let mut ctx = ProjectionContext::new("holders", options.data_quality());
    let root = fetch_holders_modules(client, symbol, MAJOR_HOLDERS_MODULE, options).await?;
    let Some(breakdown) = root.major_holders_breakdown else {
        ctx.unavailable_feature("majorHoldersBreakdown")?;
        return Ok(ctx.finish(Vec::new()));
    };

    let mut result = Vec::new();

    let insiders = breakdown.insiders.optional_raw_field(
        &mut ctx,
        "majorHoldersBreakdown.insidersPercentHeld",
        None,
        "insidersPercentHeld",
    )?;
    if let Some(value) = optional_ratio_decimal(
        &mut ctx,
        "majorHoldersBreakdown.insidersPercentHeld",
        None,
        insiders,
        "major holder percent",
    )? {
        result.push(MajorHolder {
            category: "% of Shares Held by All Insiders".into(),
            value,
        });
    }
    let institutions = breakdown.institutions.optional_raw_field(
        &mut ctx,
        "majorHoldersBreakdown.institutionsPercentHeld",
        None,
        "institutionsPercentHeld",
    )?;
    if let Some(value) = optional_ratio_decimal(
        &mut ctx,
        "majorHoldersBreakdown.institutionsPercentHeld",
        None,
        institutions,
        "major holder percent",
    )? {
        result.push(MajorHolder {
            category: "% of Shares Held by Institutions".into(),
            value,
        });
    }
    let institutions_float = breakdown.institutions_float.optional_raw_field(
        &mut ctx,
        "majorHoldersBreakdown.institutionsFloatPercentHeld",
        None,
        "institutionsFloatPercentHeld",
    )?;
    if let Some(value) = optional_ratio_decimal(
        &mut ctx,
        "majorHoldersBreakdown.institutionsFloatPercentHeld",
        None,
        institutions_float,
        "major holder percent",
    )? {
        result.push(MajorHolder {
            category: "% of Float Held by Institutions".into(),
            value,
        });
    }
    Ok(ctx.finish(result))
}

fn holder_diag_key(
    value: &serde_json::Value,
    key_field: &'static str,
    fallback: &'static str,
    idx: usize,
) -> String {
    value
        .get(key_field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map_or_else(|| format!("{fallback}[{idx}]"), ToString::to_string)
}

fn raw_field_present(value: &serde_json::Value, field: &'static str) -> bool {
    value
        .get(field)
        .and_then(|value| value.get("raw"))
        .is_some_and(|value| !value.is_null())
}

async fn map_ownership_list(
    client: &YfClient,
    symbol: &str,
    node: Option<OwnershipNode>,
    features: OwnershipFeatureNames,
    options: &CallOptions,
    ctx: &mut ProjectionContext,
) -> Result<Vec<InstitutionalHolder>, YfError> {
    let Some(holders) = ownership_list_or_unavailable(node, features, ctx)? else {
        return Ok(Vec::new());
    };

    let currency = optional_holder_value_currency(
        client,
        symbol,
        holders.iter().any(|h| raw_field_present(h, "value")),
        options,
        ctx,
    )
    .await?;

    let mut rows = Vec::new();
    for (idx, h) in holders.into_iter().enumerate() {
        let key = Some(holder_diag_key(&h, "organization", "ownershipList", idx));
        let h = match serde_json::from_value::<InstitutionalHolderNode>(h) {
            Ok(holder) => holder,
            Err(err) => {
                ctx.dropped_item(
                    "institutional_holder",
                    key.as_deref(),
                    ProjectionIssue::InvalidField {
                        field: "holder",
                        details: err.to_string(),
                    },
                )?;
                continue;
            }
        };
        let Some(holder) =
            required_nonempty_string(ctx, "institutional_holder", "organization", h.organization)?
        else {
            continue;
        };
        let Some(date_reported) = required_wire_date(
            ctx,
            "institutional_holder",
            Some(holder.as_str()),
            "reportDate",
            &h.date_reported,
        )?
        else {
            continue;
        };
        let shares = h.shares.optional_raw_field(
            ctx,
            "ownershipList[].position",
            Some(holder.as_str()),
            "position",
        )?;
        let raw_value = h.value.optional_raw_field(
            ctx,
            "ownershipList[].value",
            Some(holder.as_str()),
            "value",
        )?;
        let value = optional_money_u64_with_currency_issue(
            ctx,
            "ownershipList[].value",
            Some(holder.as_str()),
            currency.unit.as_ref(),
            currency.issue.as_ref(),
            raw_value,
            "holder monetary value",
        )?;
        let raw_pct_held = h.pct_held.optional_raw_field(
            ctx,
            "ownershipList[].pctHeld",
            Some(holder.as_str()),
            "pctHeld",
        )?;
        let pct_held = optional_ratio_decimal(
            ctx,
            "ownershipList[].pctHeld",
            Some(holder.as_str()),
            raw_pct_held,
            "holder percent",
        )?;

        rows.push(InstitutionalHolder {
            holder,
            shares,
            date_reported,
            pct_held,
            value,
        });
    }

    Ok(rows)
}

fn ownership_list_or_unavailable(
    node: Option<OwnershipNode>,
    features: OwnershipFeatureNames,
    ctx: &mut ProjectionContext,
) -> Result<Option<Vec<serde_json::Value>>, YfError> {
    let Some(node) = node else {
        ctx.unavailable_feature(features.module)?;
        return Ok(None);
    };
    let Some(holders) = node.ownership_list else {
        ctx.unavailable_feature(features.list)?;
        return Ok(None);
    };

    Ok(Some(holders))
}

async fn resolve_trading_currency(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<crate::core::currency_resolver::ResolvedCurrency, YfError> {
    client
        .resolve_trading_currency(symbol, None, TradingCurrencyEvidence::None, options)
        .await
}

async fn optional_holder_value_currency(
    client: &YfClient,
    symbol: &str,
    needed: bool,
    options: &CallOptions,
    ctx: &mut ProjectionContext,
) -> Result<ProjectedHolderCurrency, YfError> {
    if !needed {
        return Ok(ProjectedHolderCurrency::default());
    }

    let projected = project_currency_resolution(
        ctx,
        symbol,
        CurrencyPurpose::Trading,
        None,
        resolve_trading_currency(client, symbol, options).await,
    )?;
    let issue = projected.issue().cloned();
    Ok(ProjectedHolderCurrency {
        unit: projected.into_unit().map(|unit| unit.major_unit()),
        issue,
    })
}

#[derive(Default)]
struct ProjectedHolderCurrency {
    unit: Option<ResolvedCurrencyUnit>,
    issue: Option<ProjectionIssue>,
}

pub(super) async fn institutional_holders(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<YfResponse<Vec<InstitutionalHolder>>, YfError> {
    let mut ctx = ProjectionContext::new("holders", options.data_quality());
    let root = fetch_holders_modules(client, symbol, INSTITUTION_OWNERSHIP_MODULE, options).await?;
    let rows = map_ownership_list(
        client,
        symbol,
        root.institution_ownership,
        INSTITUTION_OWNERSHIP,
        options,
        &mut ctx,
    )
    .await?;
    Ok(ctx.finish(rows))
}

pub(super) async fn mutual_fund_holders(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<YfResponse<Vec<InstitutionalHolder>>, YfError> {
    let mut ctx = ProjectionContext::new("holders", options.data_quality());
    let root = fetch_holders_modules(client, symbol, FUND_OWNERSHIP_MODULE, options).await?;
    let rows = map_ownership_list(
        client,
        symbol,
        root.fund_ownership,
        FUND_OWNERSHIP,
        options,
        &mut ctx,
    )
    .await?;
    Ok(ctx.finish(rows))
}

#[allow(clippy::too_many_lines)]
pub(super) async fn insider_transactions(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<YfResponse<Vec<InsiderTransaction>>, YfError> {
    let mut ctx = ProjectionContext::new("holders", options.data_quality());
    let root = fetch_holders_modules(client, symbol, INSIDER_TRANSACTIONS_MODULE, options).await?;
    let Some(insider_transactions) = root.insider_transactions else {
        ctx.unavailable_feature("insiderTransactions")?;
        return Ok(ctx.finish(Vec::new()));
    };
    let Some(transactions) = insider_transactions.transactions else {
        ctx.unavailable_feature("insiderTransactions.transactions")?;
        return Ok(ctx.finish(Vec::new()));
    };

    let currency = optional_holder_value_currency(
        client,
        symbol,
        transactions.iter().any(|t| raw_field_present(t, "value")),
        options,
        &mut ctx,
    )
    .await?;

    let mut rows = Vec::new();
    for (idx, t) in transactions.into_iter().enumerate() {
        let key = Some(holder_diag_key(&t, "filerName", "transactions", idx));
        let t = match serde_json::from_value::<InsiderTransactionNode>(t) {
            Ok(transaction) => transaction,
            Err(err) => {
                ctx.dropped_item(
                    "insider_transaction",
                    key.as_deref(),
                    ProjectionIssue::InvalidField {
                        field: "transaction",
                        details: err.to_string(),
                    },
                )?;
                continue;
            }
        };
        let inferred_transaction_type = infer_blank_insider_transaction_type(&t);
        let Some(insider) =
            required_nonempty_string(&mut ctx, "insider_transaction", "insider", t.insider)?
        else {
            continue;
        };
        let Some(position) = required_parsed::<InsiderPosition>(
            &mut ctx,
            "insider_transaction",
            Some(insider.as_str()),
            "position",
            t.position.as_deref(),
            string_to_insider_position,
        )?
        else {
            continue;
        };
        let transaction_type = if let Some(transaction_type) = inferred_transaction_type {
            transaction_type
        } else {
            let Some(transaction_type) = required_parsed::<TransactionType>(
                &mut ctx,
                "insider_transaction",
                Some(insider.as_str()),
                "transaction",
                t.transaction.as_deref(),
                string_to_transaction_type,
            )?
            else {
                continue;
            };
            transaction_type
        };
        let Some(transaction_date) = required_wire_date(
            &mut ctx,
            "insider_transaction",
            Some(insider.as_str()),
            "startDate",
            &t.start_date,
        )?
        else {
            continue;
        };
        let shares = t.shares.optional_raw_field(
            &mut ctx,
            "insiderTransactions.transactions[].shares",
            Some(insider.as_str()),
            "shares",
        )?;
        let raw_value = t.value.optional_raw_field(
            &mut ctx,
            "insiderTransactions.transactions[].value",
            Some(insider.as_str()),
            "value",
        )?;
        let value = optional_money_u64_with_currency_issue(
            &mut ctx,
            "insiderTransactions.transactions[].value",
            Some(insider.as_str()),
            currency.unit.as_ref(),
            currency.issue.as_ref(),
            raw_value,
            "holder monetary value",
        )?;

        rows.push(InsiderTransaction {
            insider,
            position,
            transaction_type,
            shares,
            value,
            transaction_date,
            url: nonempty_string(t.url),
        });
    }

    Ok(ctx.finish(rows))
}

fn infer_blank_insider_transaction_type(t: &InsiderTransactionNode) -> Option<TransactionType> {
    let blank_text = t
        .transaction
        .as_deref()
        .is_none_or(|transaction| transaction.trim().is_empty());
    let positive_shares = t
        .shares
        .as_ref()
        .and_then(|shares| shares.raw)
        .is_some_and(|shares| shares > 0);
    let no_value = t.value.as_ref().and_then(|value| value.raw).is_none();

    if blank_text && positive_shares && no_value {
        Some(TransactionType::Exercise)
    } else {
        None
    }
}

pub(super) async fn insider_roster_holders(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<YfResponse<Vec<InsiderRosterHolder>>, YfError> {
    let mut ctx = ProjectionContext::new("holders", options.data_quality());
    let root = fetch_holders_modules(client, symbol, INSIDER_HOLDERS_MODULE, options).await?;
    let Some(insider_holders) = root.insider_holders else {
        ctx.unavailable_feature("insiderHolders")?;
        return Ok(ctx.finish(Vec::new()));
    };
    let Some(holders) = insider_holders.holders else {
        ctx.unavailable_feature("insiderHolders.holders")?;
        return Ok(ctx.finish(Vec::new()));
    };

    let mut rows = Vec::new();
    for (idx, h) in holders.into_iter().enumerate() {
        let key = Some(holder_diag_key(&h, "name", "holders", idx));
        let h = match serde_json::from_value::<InsiderRosterHolderNode>(h) {
            Ok(holder) => holder,
            Err(err) => {
                ctx.dropped_item(
                    "insider_roster_holder",
                    key.as_deref(),
                    ProjectionIssue::InvalidField {
                        field: "holder",
                        details: err.to_string(),
                    },
                )?;
                continue;
            }
        };
        let Some(name) =
            required_nonempty_string(&mut ctx, "insider_roster_holder", "name", h.name)?
        else {
            continue;
        };
        let Some(position) = required_parsed::<InsiderPosition>(
            &mut ctx,
            "insider_roster_holder",
            Some(name.as_str()),
            "relation",
            h.relation.as_deref(),
            string_to_insider_position,
        )?
        else {
            continue;
        };
        let Some(most_recent_transaction) = required_parsed::<TransactionType>(
            &mut ctx,
            "insider_roster_holder",
            Some(name.as_str()),
            "transactionDescription",
            h.most_recent_transaction.as_deref(),
            string_to_transaction_type,
        )?
        else {
            continue;
        };
        let Some(latest_transaction_date) = required_wire_date(
            &mut ctx,
            "insider_roster_holder",
            Some(name.as_str()),
            "latestTransDate",
            &h.latest_transaction_date,
        )?
        else {
            continue;
        };
        let Some(position_direct_date) = required_wire_date(
            &mut ctx,
            "insider_roster_holder",
            Some(name.as_str()),
            "positionDirectDate",
            &h.position_direct_date,
        )?
        else {
            continue;
        };
        let shares_owned_directly = h.shares_owned_directly.optional_raw_field(
            &mut ctx,
            "insiderHolders.holders[].positionDirect",
            Some(name.as_str()),
            "positionDirect",
        )?;

        rows.push(InsiderRosterHolder {
            name,
            position,
            most_recent_transaction,
            latest_transaction_date,
            shares_owned_directly,
            position_direct_date,
        });
    }

    Ok(ctx.finish(rows))
}

pub(super) async fn net_share_purchase_activity(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<YfResponse<Option<NetSharePurchaseActivity>>, YfError> {
    let mut ctx = ProjectionContext::new("holders", options.data_quality());
    let root =
        fetch_holders_modules(client, symbol, NET_SHARE_PURCHASE_ACTIVITY_MODULE, options).await?;
    let Some(n) = root.net_share_purchase_activity else {
        ctx.unavailable_feature("netSharePurchaseActivity")?;
        return Ok(ctx.finish(None));
    };

    let period_key = n.period.clone();
    let Some(period) = required_parsed(
        &mut ctx,
        "net_share_purchase_activity",
        period_key.as_deref(),
        "period",
        n.period.as_deref(),
        crate::core::conversions::string_to_period,
    )?
    else {
        return Ok(ctx.finish(None));
    };

    let net_percent_insider_shares = n.net_percent_insider_shares.optional_raw_field(
        &mut ctx,
        "netSharePurchaseActivity.netPercentInsiderShares",
        period_key.as_deref(),
        "netPercentInsiderShares",
    )?;

    let data = Some(NetSharePurchaseActivity {
        period,
        buy_shares: n.buy_info_shares.optional_raw_field(
            &mut ctx,
            "netSharePurchaseActivity.buyInfoShares",
            period_key.as_deref(),
            "buyInfoShares",
        )?,
        buy_count: n.buy_info_count.optional_raw_field(
            &mut ctx,
            "netSharePurchaseActivity.buyInfoCount",
            period_key.as_deref(),
            "buyInfoCount",
        )?,
        sell_shares: n.sell_info_shares.optional_raw_field(
            &mut ctx,
            "netSharePurchaseActivity.sellInfoShares",
            period_key.as_deref(),
            "sellInfoShares",
        )?,
        sell_count: n.sell_info_count.optional_raw_field(
            &mut ctx,
            "netSharePurchaseActivity.sellInfoCount",
            period_key.as_deref(),
            "sellInfoCount",
        )?,
        net_shares: n.net_info_shares.optional_raw_field(
            &mut ctx,
            "netSharePurchaseActivity.netInfoShares",
            period_key.as_deref(),
            "netInfoShares",
        )?,
        net_count: n.net_info_count.optional_raw_field(
            &mut ctx,
            "netSharePurchaseActivity.netInfoCount",
            period_key.as_deref(),
            "netInfoCount",
        )?,
        total_insider_shares: n.total_insider_shares.optional_raw_field(
            &mut ctx,
            "netSharePurchaseActivity.totalInsiderShares",
            period_key.as_deref(),
            "totalInsiderShares",
        )?,
        net_percent_insider_shares,
    });
    Ok(ctx.finish(data))
}

fn optional_ratio_decimal(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    value: Option<Decimal>,
    target: &'static str,
) -> Result<Option<Ratio>, YfError> {
    optional_projected(ctx, path, key, value, |value| {
        Ratio::new(value).map_err(|_| ProjectionIssue::ConversionFailed { target })
    })
}
