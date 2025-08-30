use chrono::{Duration, Utc};
use std::collections::BTreeMap;

use crate::{
    ShareCount,
    core::{
        YfClient, YfError,
        client::{CacheMode, RetryConfig},
        wire::{from_raw, from_raw_date},
    },
    fundamentals::wire::{TimeseriesData, TimeseriesEnvelope},
};

use super::fetch::fetch_modules;
use super::{
    BalanceSheetRow, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps, EarningsYear,
    IncomeStatementRow,
};

pub(super) async fn income_statement(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<IncomeStatementRow>, YfError> {
    let modules = if quarterly {
        "incomeStatementHistoryQuarterly"
    } else {
        "incomeStatementHistory"
    };

    let root = fetch_modules(client, symbol, modules, cache_mode, retry_override).await?;
    let arr = if quarterly {
        root.income_statement_history_quarterly
            .and_then(|x| x.income_statement_history)
    } else {
        root.income_statement_history
            .and_then(|x| x.income_statement_history)
    }
    .unwrap_or_default();

    Ok(arr
        .into_iter()
        .map(|n| IncomeStatementRow {
            period_end: from_raw_date(n.end_date).unwrap_or(0),
            total_revenue: from_raw(n.total_revenue),
            gross_profit: from_raw(n.gross_profit),
            operating_income: from_raw(n.operating_income),
            net_income: from_raw(n.net_income),
        })
        .collect())
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::cognitive_complexity)]
pub(super) async fn balance_sheet(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<BalanceSheetRow>, YfError> {
    use serde::Deserialize;

    use crate::core::wire::{RawNum, RawNumU64};

    #[derive(Deserialize)]
    struct TimeseriesValueF64 {
        #[serde(rename = "reportedValue")]
        reported_value: Option<RawNum<f64>>,
    }
    #[derive(Deserialize)]
    struct TimeseriesValueU64 {
        #[serde(rename = "reportedValue")]
        reported_value: Option<RawNumU64>,
    }

    let prefix = if quarterly { "quarterly" } else { "annual" };
    let keys = [
        "TotalAssets",
        "TotalLiabilitiesNetMinorityInterest",
        "StockholdersEquity",
        "CashAndCashEquivalents",
        "LongTermDebt",
        "OrdinarySharesNumber",
    ];
    let types: Vec<String> = keys.iter().map(|k| format!("{prefix}{k}")).collect();
    let type_str = types.join(",");

    let end_ts = Utc::now().timestamp();
    let start_ts = Utc::now()
        .checked_sub_signed(Duration::days(365 * 5))
        .map_or(0, |dt| dt.timestamp());

    let mut url = client.base_timeseries().join(symbol)?;
    url.query_pairs_mut()
        .append_pair("symbol", symbol)
        .append_pair("type", &type_str)
        .append_pair("period1", &start_ts.to_string())
        .append_pair("period2", &end_ts.to_string());

    client.ensure_credentials().await?;
    if let Some(crumb) = client.crumb().await {
        url.query_pairs_mut().append_pair("crumb", &crumb);
    }

    let body = if cache_mode == CacheMode::Use {
        if let Some(cached) = client.cache_get(&url).await {
            cached
        } else {
            let resp = client
                .send_with_retry(client.http().get(url.clone()), retry_override)
                .await?;
            let endpoint = format!("timeseries_balance_sheet_{prefix}");
            let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
            if cache_mode != CacheMode::Bypass {
                client.cache_put(&url, &text, None).await;
            }
            text
        }
    } else {
        let resp = client
            .send_with_retry(client.http().get(url.clone()), retry_override)
            .await?;
        let endpoint = format!("timeseries_balance_sheet_{prefix}");
        let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
        if cache_mode != CacheMode::Bypass {
            client.cache_put(&url, &text, None).await;
        }
        text
    };

    let envelope: TimeseriesEnvelope = serde_json::from_str(&body)
        .map_err(|e| YfError::Data(format!("balance sheet timeseries json parse: {e}")))?;

    let result_vec = envelope
        .timeseries
        .and_then(|ts| ts.result)
        .unwrap_or_default();

    if result_vec.is_empty() {
        return Ok(vec![]);
    }

    let mut rows_map = BTreeMap::<i64, BalanceSheetRow>::new();

    for item in result_vec {
        if let (Some(timestamps), Some((key, values_json))) =
            (item.timestamp, item.values.into_iter().next())
        {
            if key.ends_with("OrdinarySharesNumber") {
                if let Ok(values) = serde_json::from_value::<Vec<TimeseriesValueU64>>(values_json) {
                    for (i, ts) in timestamps.iter().enumerate() {
                        let row = rows_map.entry(*ts).or_insert_with(|| BalanceSheetRow {
                            period_end: *ts,
                            total_assets: None,
                            total_liabilities: None,
                            total_equity: None,
                            cash: None,
                            long_term_debt: None,
                            shares_outstanding: None,
                        });
                        row.shares_outstanding = values
                            .get(i)
                            .and_then(|v| v.reported_value.and_then(|rv| rv.raw));
                    }
                }
            } else if let Ok(values) =
                serde_json::from_value::<Vec<TimeseriesValueF64>>(values_json)
            {
                for (i, ts) in timestamps.iter().enumerate() {
                    let row = rows_map.entry(*ts).or_insert_with(|| BalanceSheetRow {
                        period_end: *ts,
                        total_assets: None,
                        total_liabilities: None,
                        total_equity: None,
                        cash: None,
                        long_term_debt: None,
                        shares_outstanding: None,
                    });

                    let value = values
                        .get(i)
                        .and_then(|v| v.reported_value.and_then(|rv| rv.raw));

                    if key == format!("{prefix}TotalAssets") {
                        row.total_assets = value;
                    } else if key == format!("{prefix}TotalLiabilitiesNetMinorityInterest") {
                        row.total_liabilities = value;
                    } else if key == format!("{prefix}StockholdersEquity") {
                        row.total_equity = value;
                    } else if key == format!("{prefix}CashAndCashEquivalents") {
                        row.cash = value;
                    } else if key == format!("{prefix}LongTermDebt") {
                        row.long_term_debt = value;
                    }
                }
            }
        }
    }

    Ok(rows_map.into_values().rev().collect())
}

#[allow(clippy::too_many_lines)]
pub(super) async fn cashflow(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<CashflowRow>, YfError> {
    use serde::Deserialize;

    use crate::core::wire::RawNum;

    #[derive(Deserialize)]
    struct TimeseriesValueF64 {
        #[serde(rename = "reportedValue")]
        reported_value: Option<RawNum<f64>>,
    }

    let prefix = if quarterly { "quarterly" } else { "annual" };
    let keys = [
        "OperatingCashFlow",
        "CapitalExpenditure",
        "FreeCashFlow",
        "NetIncome",
    ];
    let types: Vec<String> = keys.iter().map(|k| format!("{prefix}{k}")).collect();
    let type_str = types.join(",");

    let end_ts = Utc::now().timestamp();
    let start_ts = Utc::now()
        .checked_sub_signed(Duration::days(365 * 5))
        .map_or(0, |dt| dt.timestamp());

    let mut url = client.base_timeseries().join(symbol)?;
    url.query_pairs_mut()
        .append_pair("symbol", symbol)
        .append_pair("type", &type_str)
        .append_pair("period1", &start_ts.to_string())
        .append_pair("period2", &end_ts.to_string());

    client.ensure_credentials().await?;
    if let Some(crumb) = client.crumb().await {
        url.query_pairs_mut().append_pair("crumb", &crumb);
    }

    let body = if cache_mode == CacheMode::Use {
        if let Some(cached) = client.cache_get(&url).await {
            cached
        } else {
            let resp = client
                .send_with_retry(client.http().get(url.clone()), retry_override)
                .await?;
            let endpoint = format!("timeseries_cash_flow_{prefix}");
            let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
            if cache_mode != CacheMode::Bypass {
                client.cache_put(&url, &text, None).await;
            }
            text
        }
    } else {
        let resp = client
            .send_with_retry(client.http().get(url.clone()), retry_override)
            .await?;
        let endpoint = format!("timeseries_cash_flow_{prefix}");
        let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
        if cache_mode != CacheMode::Bypass {
            client.cache_put(&url, &text, None).await;
        }
        text
    };

    let envelope: TimeseriesEnvelope = serde_json::from_str(&body)
        .map_err(|e| YfError::Data(format!("cash flow timeseries json parse: {e}")))?;

    let result_vec = envelope
        .timeseries
        .and_then(|ts| ts.result)
        .unwrap_or_default();

    if result_vec.is_empty() {
        return Ok(vec![]);
    }

    let mut rows_map = BTreeMap::<i64, CashflowRow>::new();

    for item in result_vec {
        if let (Some(timestamps), Some((key, values_json))) =
            (item.timestamp, item.values.into_iter().next())
            && let Ok(values) = serde_json::from_value::<Vec<TimeseriesValueF64>>(values_json)
        {
            for (i, ts) in timestamps.iter().enumerate() {
                let row = rows_map.entry(*ts).or_insert_with(|| CashflowRow {
                    period_end: *ts,
                    operating_cashflow: None,
                    capital_expenditures: None,
                    free_cash_flow: None,
                    net_income: None,
                });

                let value = values
                    .get(i)
                    .and_then(|v| v.reported_value.and_then(|rv| rv.raw));

                if key == format!("{prefix}OperatingCashFlow") {
                    row.operating_cashflow = value;
                } else if key == format!("{prefix}CapitalExpenditure") {
                    row.capital_expenditures = value;
                } else if key == format!("{prefix}FreeCashFlow") {
                    row.free_cash_flow = value;
                } else if key == format!("{prefix}NetIncome") {
                    row.net_income = value;
                }
            }
        }
    }

    // After filling values, calculate FCF if it's missing.
    for row in rows_map.values_mut() {
        if row.free_cash_flow.is_none()
            && let (Some(ocf), Some(capex)) = (row.operating_cashflow, row.capital_expenditures)
        {
            // In timeseries API, capex is negative for cash outflow.
            row.free_cash_flow = Some(ocf + capex);
        }
    }

    Ok(rows_map.into_values().rev().collect())
}

pub(super) async fn earnings(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Earnings, YfError> {
    let root = fetch_modules(client, symbol, "earnings", cache_mode, retry_override).await?;
    let e = root
        .earnings
        .ok_or_else(|| YfError::Data("earnings missing".into()))?;

    let yearly = e
        .financials_chart
        .as_ref()
        .and_then(|fc| fc.yearly.as_ref())
        .map(|v| {
            v.iter()
                .map(|y| EarningsYear {
                    year: i32::try_from(y.date.unwrap_or(0)).unwrap_or(0),
                    revenue: y.revenue.as_ref().and_then(|x| x.raw),
                    earnings: y.earnings.as_ref().and_then(|x| x.raw),
                })
                .collect()
        })
        .unwrap_or_default();

    let quarterly = e
        .financials_chart
        .as_ref()
        .and_then(|fc| fc.quarterly.as_ref())
        .map(|v| {
            v.iter()
                .map(|q| EarningsQuarter {
                    period: q.date.clone().unwrap_or_default(),
                    revenue: q.revenue.as_ref().and_then(|x| x.raw),
                    earnings: q.earnings.as_ref().and_then(|x| x.raw),
                })
                .collect()
        })
        .unwrap_or_default();

    let quarterly_eps = e
        .earnings_chart
        .as_ref()
        .and_then(|ec| ec.quarterly.as_ref())
        .map(|v| {
            v.iter()
                .map(|q| EarningsQuarterEps {
                    period: q.date.clone().unwrap_or_default(),
                    actual: q.actual.as_ref().and_then(|x| x.raw),
                    estimate: q.estimate.as_ref().and_then(|x| x.raw),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Earnings {
        yearly,
        quarterly,
        quarterly_eps,
    })
}

pub(super) async fn calendar(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<super::Calendar, YfError> {
    let root = fetch_modules(client, symbol, "calendarEvents", cache_mode, retry_override).await?;
    let c = root
        .calendar_events
        .and_then(|ce| ce.earnings)
        .ok_or_else(|| YfError::Data("calendarEvents.earnings missing".into()))?;

    let earnings_dates = c
        .earnings_date
        .unwrap_or_default()
        .into_iter()
        .filter_map(|d| d.raw)
        .collect();

    Ok(super::Calendar {
        earnings_dates,
        ex_dividend_date: c.ex_dividend_date.and_then(|x| x.raw),
        dividend_date: c.dividend_date.and_then(|x| x.raw),
    })
}

pub(super) async fn shares(
    client: &YfClient,
    symbol: &str,
    start: Option<chrono::DateTime<Utc>>,
    end: Option<chrono::DateTime<Utc>>,
    quarterly: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<ShareCount>, YfError> {
    let end_ts = end.unwrap_or_else(Utc::now).timestamp();
    let start_ts = start
        .unwrap_or_else(|| Utc::now() - Duration::days(548))
        .timestamp();

    let type_key = if quarterly {
        "quarterlyBasicAverageShares"
    } else {
        "annualBasicAverageShares"
    };

    let mut url = client.base_timeseries().join(symbol)?;
    url.query_pairs_mut()
        .append_pair("symbol", symbol)
        .append_pair("type", type_key)
        .append_pair("period1", &start_ts.to_string())
        .append_pair("period2", &end_ts.to_string());

    client.ensure_credentials().await?;
    if let Some(crumb) = client.crumb().await {
        url.query_pairs_mut().append_pair("crumb", &crumb);
    }

    let body = if cache_mode == CacheMode::Use {
        if let Some(cached) = client.cache_get(&url).await {
            cached
        } else {
            let resp = client
                .send_with_retry(client.http().get(url.clone()), retry_override)
                .await?;
            let endpoint = format!("timeseries_{type_key}");
            let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
            if cache_mode != CacheMode::Bypass {
                client.cache_put(&url, &text, None).await;
            }
            text
        }
    } else {
        let resp = client
            .send_with_retry(client.http().get(url.clone()), retry_override)
            .await?;
        let endpoint = format!("timeseries_{type_key}");
        let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
        if cache_mode != CacheMode::Bypass {
            client.cache_put(&url, &text, None).await;
        }
        text
    };

    let envelope: TimeseriesEnvelope = serde_json::from_str(&body)
        .map_err(|e| YfError::Data(format!("shares timeseries json parse: {e}")))?;

    let result_data: Option<TimeseriesData> = envelope
        .timeseries
        .and_then(|ts| ts.result)
        .and_then(|mut v| v.pop());

    let Some(TimeseriesData {
        timestamp: Some(timestamps),
        values: mut values_map,
        ..
    }) = result_data
    else {
        return Ok(vec![]);
    };

    let Some(values_json) = values_map.remove(type_key) else {
        return Ok(vec![]);
    };

    let values: Vec<super::wire::TimeseriesValue> = serde_json::from_value(values_json)
        .map_err(|e| YfError::Data(format!("shares timeseries values parse: {e}")))?;

    let counts = timestamps
        .into_iter()
        .zip(values.into_iter())
        .filter_map(|(ts, val)| {
            val.reported_value
                .and_then(|rv| rv.raw)
                .map(|shares| ShareCount { date: ts, shares })
        })
        .collect();

    Ok(counts)
}
