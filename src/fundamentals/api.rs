use crate::core::{YfClient, YfError};

use super::fetch::fetch_modules;
use super::wire::raw_num;
use super::{
    BalanceSheetRow, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps, EarningsYear,
    IncomeStatementRow,
};

/* ---------- Public entry points (mapping wire → public models) ---------- */

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

    let root = fetch_modules(client, symbol, modules).await?;
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

    let root = fetch_modules(client, symbol, modules).await?;
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

    let root = fetch_modules(client, symbol, modules).await?;
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
    let root = fetch_modules(client, symbol, "earnings").await?;
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
    let root = fetch_modules(client, symbol, "calendarEvents").await?;
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
