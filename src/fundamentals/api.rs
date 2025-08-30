use chrono::{Duration, Utc};

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

pub(super) async fn balance_sheet(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<BalanceSheetRow>, YfError> {
    let modules = if quarterly {
        "balanceSheetHistoryQuarterly"
    } else {
        "balanceSheetHistory"
    };

    let root = fetch_modules(client, symbol, modules, cache_mode, retry_override).await?;
    let arr = if quarterly {
        root.balance_sheet_history_quarterly
            .and_then(|x| x.balance_sheet_statements)
    } else {
        root.balance_sheet_history
            .and_then(|x| x.balance_sheet_statements)
    }
    .unwrap_or_default();

    Ok(arr
        .into_iter()
        .map(|n| BalanceSheetRow {
            period_end: from_raw_date(n.end_date).unwrap_or(0),
            total_assets: from_raw(n.total_assets),
            total_liabilities: from_raw(n.total_liab),
            total_equity: from_raw(n.total_stockholder_equity),
            cash: from_raw(n.cash),
            long_term_debt: from_raw(n.long_term_debt),
            shares_outstanding: from_raw(n.shares_outstanding).and_then(|v| u64::try_from(v).ok()),
        })
        .collect())
}

pub(super) async fn cashflow(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<CashflowRow>, YfError> {
    let modules = if quarterly {
        "cashflowStatementHistoryQuarterly"
    } else {
        "cashflowStatementHistory"
    };

    let root = fetch_modules(client, symbol, modules, cache_mode, retry_override).await?;
    let arr = if quarterly {
        root.cashflow_statement_history_quarterly
            .and_then(|x| x.cashflow_statements)
    } else {
        root.cashflow_statement_history
            .and_then(|x| x.cashflow_statements)
    }
    .unwrap_or_default();

    Ok(arr
        .into_iter()
        .map(|n| {
            let ocf = from_raw(n.total_cash_from_operating_activities);
            let capex = from_raw(n.capital_expenditures);
            let fcf = match (from_raw(n.free_cashflow), ocf, capex) {
                (Some(x), _, _) => Some(x),
                (None, Some(a), Some(b)) => Some(a - b),
                _ => None,
            };
            CashflowRow {
                period_end: from_raw_date(n.end_date).unwrap_or(0),
                operating_cashflow: ocf,
                capital_expenditures: capex,
                free_cash_flow: fcf,
                net_income: from_raw(n.net_income),
            }
        })
        .collect())
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

    let mut url = client.base_timeseries().join(symbol)?;

    let type_key = if quarterly {
        "quarterlyBasicAverageShares"
    } else {
        "annualBasicAverageShares"
    };

    url.query_pairs_mut()
        .append_pair("symbol", symbol)
        .append_pair("type", type_key)
        .append_pair("period1", &start_ts.to_string())
        .append_pair("period2", &end_ts.to_string());

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

    let Some(values) = values_map.remove(type_key) else {
        return Ok(vec![]);
    };

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
