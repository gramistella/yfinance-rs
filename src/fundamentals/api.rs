use serde::Deserialize;

use crate::core::net;
use crate::core::{YfClient, YfError};

use super::{
    BalanceSheetRow, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps, EarningsYear,
    IncomeStatementRow,
};

#[cfg(any(debug_assertions, feature = "debug-dumps"))]
use crate::profile::debug::debug_dump_api; // reuse helper to persist JSON if needed

/* ---------- Public entry points ---------- */

pub async fn income_statement(
    client: &mut YfClient,
    symbol: &str,
    quarterly: bool,
) -> Result<Vec<IncomeStatementRow>, YfError> {
    let modules = if quarterly {
        "incomeStatementHistoryQuarterly"
    } else {
        "incomeStatementHistory"
    };
    let env = call_quote_summary(client, symbol, modules).await?;
    let root = get_first_result(env)?;
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
            period_end: n.end_date.and_then(|d| d.raw).unwrap_or(0),
            total_revenue: n.total_revenue.and_then(raw_num),
            gross_profit: n.gross_profit.and_then(raw_num),
            operating_income: n.operating_income.and_then(raw_num),
            net_income: n.net_income.and_then(raw_num),
        })
        .collect())
}

pub async fn balance_sheet(
    client: &mut YfClient,
    symbol: &str,
    quarterly: bool,
) -> Result<Vec<BalanceSheetRow>, YfError> {
    let modules = if quarterly {
        "balanceSheetHistoryQuarterly"
    } else {
        "balanceSheetHistory"
    };
    let env = call_quote_summary(client, symbol, modules).await?;
    let root = get_first_result(env)?;
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
            period_end: n.end_date.and_then(|d| d.raw).unwrap_or(0),
            total_assets: n.total_assets.and_then(raw_num),
            total_liabilities: n.total_liab.and_then(raw_num),
            total_equity: n.total_stockholder_equity.and_then(raw_num),
            cash: n.cash.and_then(raw_num),
            long_term_debt: n.long_term_debt.and_then(raw_num),
        })
        .collect())
}

pub async fn cashflow(
    client: &mut YfClient,
    symbol: &str,
    quarterly: bool,
) -> Result<Vec<CashflowRow>, YfError> {
    let modules = if quarterly {
        "cashflowStatementHistoryQuarterly"
    } else {
        "cashflowStatementHistory"
    };
    let env = call_quote_summary(client, symbol, modules).await?;
    let root = get_first_result(env)?;
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
            let ocf = n.total_cash_from_operating_activities.and_then(raw_num);
            let capex = n.capital_expenditures.and_then(raw_num);
            let fcf = match (n.free_cashflow.and_then(raw_num), ocf, capex) {
                (Some(x), _, _) => Some(x),
                (None, Some(a), Some(b)) => Some(a - b),
                _ => None,
            };
            CashflowRow {
                period_end: n.end_date.and_then(|d| d.raw).unwrap_or(0),
                operating_cashflow: ocf,
                capital_expenditures: capex,
                free_cash_flow: fcf,
                net_income: n.net_income.and_then(raw_num),
            }
        })
        .collect())
}

pub async fn earnings(client: &mut YfClient, symbol: &str) -> Result<Earnings, YfError> {
    let env = call_quote_summary(client, symbol, "earnings").await?;
    let root = get_first_result(env)?;
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
                    year: y.date.unwrap_or(0) as i32,
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

pub async fn calendar(client: &mut YfClient, symbol: &str) -> Result<super::Calendar, YfError> {
    let env = call_quote_summary(client, symbol, "calendarEvents").await?;
    let root = get_first_result(env)?;
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

/* ---------- Shared call + serde skeleton ---------- */

async fn call_quote_summary(
    client: &mut YfClient,
    symbol: &str,
    modules: &str,
) -> Result<V10Envelope, YfError> {
    // Ensure we have credentials + crumb, and retry once on "Invalid Crumb"
    for attempt in 0..=1 {
        client.ensure_credentials().await?;

        let crumb = client
            .crumb()
            .ok_or_else(|| YfError::Data("Crumb is not set".into()))?
            .to_string();

        let mut url = client.base_quote_api().join(symbol)?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("modules", modules);
            qp.append_pair("crumb", &crumb);
        }

        let resp = client.http().get(url.clone()).send().await?;
        let text = net::get_text(resp, "fundamentals_api", symbol, "json").await?;
        #[cfg(any(debug_assertions, feature = "debug-dumps"))]
        {
            let _ = debug_dump_api(symbol, &text);
        }

        let env: V10Envelope = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => return Err(YfError::Data(format!("quoteSummary json parse: {e}"))),
        };

        if let Some(error) = env.quote_summary.as_ref().and_then(|qs| qs.error.as_ref()) {
            let desc = error.description.to_ascii_lowercase();
            if desc.contains("invalid crumb") && attempt == 0 {
                if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                    eprintln!("YF_DEBUG: Invalid crumb in fundamentals; refreshing and retrying.");
                }
                client.clear_crumb();
                continue;
            }
            return Err(YfError::Data(format!("yahoo error: {}", error.description)));
        }

        return Ok(env);
    }

    Err(YfError::Data(
        "fundamentals API call failed after retry".into(),
    ))
}

fn get_first_result(env: V10Envelope) -> Result<V10Result, YfError> {
    env.quote_summary
        .and_then(|qs| qs.result)
        .and_then(|mut v| v.pop())
        .ok_or_else(|| YfError::Data("empty quoteSummary result".into()))
}

fn raw_num(n: RawNum) -> Option<f64> {
    // Accept ints or floats under `raw`; serde maps to f64 already
    n.raw
}

/* ---------------- Serde mapping (only what we need) ---------------- */

#[derive(Deserialize)]
struct V10Envelope {
    #[serde(rename = "quoteSummary")]
    quote_summary: Option<V10QuoteSummary>,
}

#[derive(Deserialize)]
struct V10QuoteSummary {
    result: Option<Vec<V10Result>>,
    error: Option<V10Error>,
}

#[derive(Deserialize)]
struct V10Error {
    description: String,
}

#[derive(Deserialize)]
struct V10Result {
    // Income statement
    #[serde(rename = "incomeStatementHistory")]
    income_statement_history: Option<IncomeHistoryNode>,
    #[serde(rename = "incomeStatementHistoryQuarterly")]
    income_statement_history_quarterly: Option<IncomeHistoryNode>,

    // Balance sheet
    #[serde(rename = "balanceSheetHistory")]
    balance_sheet_history: Option<BalanceHistoryNode>,
    #[serde(rename = "balanceSheetHistoryQuarterly")]
    balance_sheet_history_quarterly: Option<BalanceHistoryNode>,

    // Cashflow
    #[serde(rename = "cashflowStatementHistory")]
    cashflow_statement_history: Option<CashflowHistoryNode>,
    #[serde(rename = "cashflowStatementHistoryQuarterly")]
    cashflow_statement_history_quarterly: Option<CashflowHistoryNode>,

    // Earnings + Calendar
    earnings: Option<EarningsNode>,
    #[serde(rename = "calendarEvents")]
    calendar_events: Option<CalendarEventsNode>,
}

/* --- income --- */
#[derive(Deserialize)]
struct IncomeHistoryNode {
    #[serde(rename = "incomeStatementHistory")]
    income_statement_history: Option<Vec<IncomeRowNode>>,
}

#[derive(Deserialize)]
struct IncomeRowNode {
    #[serde(rename = "endDate")]
    end_date: Option<RawDate>,
    #[serde(rename = "totalRevenue")]
    total_revenue: Option<RawNum>,
    #[serde(rename = "grossProfit")]
    gross_profit: Option<RawNum>,
    #[serde(rename = "operatingIncome")]
    operating_income: Option<RawNum>,
    #[serde(rename = "netIncome")]
    net_income: Option<RawNum>,
}

/* --- balance --- */
#[derive(Deserialize)]
struct BalanceHistoryNode {
    #[serde(rename = "balanceSheetStatements")]
    balance_sheet_statements: Option<Vec<BalanceRowNode>>,
}

#[derive(Deserialize)]
struct BalanceRowNode {
    #[serde(rename = "endDate")]
    end_date: Option<RawDate>,

    #[serde(rename = "totalAssets")]
    total_assets: Option<RawNum>,
    #[serde(rename = "totalLiab")]
    total_liab: Option<RawNum>,
    #[serde(rename = "totalStockholderEquity")]
    total_stockholder_equity: Option<RawNum>,

    cash: Option<RawNum>,

    #[serde(rename = "longTermDebt")]
    long_term_debt: Option<RawNum>,
}

/* --- cashflow --- */
#[derive(Deserialize)]
struct CashflowHistoryNode {
    #[serde(rename = "cashflowStatements")]
    cashflow_statements: Option<Vec<CashflowRowNode>>,
}

#[derive(Deserialize)]
struct CashflowRowNode {
    #[serde(rename = "endDate")]
    end_date: Option<RawDate>,

    #[serde(rename = "totalCashFromOperatingActivities")]
    total_cash_from_operating_activities: Option<RawNum>,

    #[serde(rename = "capitalExpenditures")]
    capital_expenditures: Option<RawNum>,

    #[serde(rename = "freeCashflow")]
    free_cashflow: Option<RawNum>,

    #[serde(rename = "netIncome")]
    net_income: Option<RawNum>,
}

/* --- earnings --- */
#[derive(Deserialize)]
struct EarningsNode {
    #[serde(rename = "financialsChart")]
    financials_chart: Option<FinancialsChartNode>,
    #[serde(rename = "earningsChart")]
    earnings_chart: Option<EarningsChartNode>,
}

#[derive(Deserialize)]
struct FinancialsChartNode {
    yearly: Option<Vec<FinancialYearNode>>,
    quarterly: Option<Vec<FinancialQuarterNode>>,
}

#[derive(Deserialize)]
struct FinancialYearNode {
    date: Option<i64>, // year like 2024
    revenue: Option<RawNum>,
    earnings: Option<RawNum>,
}

#[derive(Deserialize)]
struct FinancialQuarterNode {
    date: Option<String>, // e.g. "2024Q1"
    revenue: Option<RawNum>,
    earnings: Option<RawNum>,
}

#[derive(Deserialize)]
struct EarningsChartNode {
    quarterly: Option<Vec<EpsQuarterNode>>,
}

#[derive(Deserialize)]
struct EpsQuarterNode {
    date: Option<String>,
    actual: Option<RawNum>,
    estimate: Option<RawNum>,
}

/* --- calendar --- */
#[derive(Deserialize)]
struct CalendarEventsNode {
    earnings: Option<CalendarEarningsNode>,
}

#[derive(Deserialize)]
struct CalendarEarningsNode {
    #[serde(rename = "earningsDate")]
    earnings_date: Option<Vec<RawDate>>,
    #[serde(rename = "exDividendDate")]
    ex_dividend_date: Option<RawDate>,
    #[serde(rename = "dividendDate")]
    dividend_date: Option<RawDate>,
}

/* --- shared small wrappers --- */

#[derive(Deserialize, Clone, Copy)]
struct RawDate {
    raw: Option<i64>,
}

#[derive(Deserialize, Clone, Copy)]
struct RawNum {
    raw: Option<f64>,
}
