use serde::Deserialize;

/* ---------------- Serde mapping (only what we need) ---------------- */

#[derive(Deserialize)]
pub(crate) struct V10Result {
    /* income */
    #[serde(rename = "incomeStatementHistory")]
    pub(crate) income_statement_history: Option<IncomeHistoryNode>,
    #[serde(rename = "incomeStatementHistoryQuarterly")]
    pub(crate) income_statement_history_quarterly: Option<IncomeHistoryNode>,

    /* balance */
    #[serde(rename = "balanceSheetHistory")]
    pub(crate) balance_sheet_history: Option<BalanceHistoryNode>,
    #[serde(rename = "balanceSheetHistoryQuarterly")]
    pub(crate) balance_sheet_history_quarterly: Option<BalanceHistoryNode>,

    /* cashflow */
    #[serde(rename = "cashflowStatementHistory")]
    pub(crate) cashflow_statement_history: Option<CashflowHistoryNode>,
    #[serde(rename = "cashflowStatementHistoryQuarterly")]
    pub(crate) cashflow_statement_history_quarterly: Option<CashflowHistoryNode>,

    /* earnings + calendar */
    pub(crate) earnings: Option<EarningsNode>,
    #[serde(rename = "calendarEvents")]
    pub(crate) calendar_events: Option<CalendarEventsNode>,
}

/* --- income --- */
#[derive(Deserialize)]
pub(crate) struct IncomeHistoryNode {
    #[serde(rename = "incomeStatementHistory")]
    pub(crate) income_statement_history: Option<Vec<IncomeRowNode>>,
}

#[derive(Deserialize)]
pub(crate) struct IncomeRowNode {
    #[serde(rename = "endDate")]
    pub(crate) end_date: Option<RawDate>,
    #[serde(rename = "totalRevenue")]
    pub(crate) total_revenue: Option<RawNum>,
    #[serde(rename = "grossProfit")]
    pub(crate) gross_profit: Option<RawNum>,
    #[serde(rename = "operatingIncome")]
    pub(crate) operating_income: Option<RawNum>,
    #[serde(rename = "netIncome")]
    pub(crate) net_income: Option<RawNum>,
}

/* --- balance --- */
#[derive(Deserialize)]
pub(crate) struct BalanceHistoryNode {
    #[serde(rename = "balanceSheetStatements")]
    pub(crate) balance_sheet_statements: Option<Vec<BalanceRowNode>>,
}

#[derive(Deserialize)]
pub(crate) struct BalanceRowNode {
    #[serde(rename = "endDate")]
    pub(crate) end_date: Option<RawDate>,

    #[serde(rename = "totalAssets")]
    pub(crate) total_assets: Option<RawNum>,
    #[serde(rename = "totalLiab")]
    pub(crate) total_liab: Option<RawNum>,
    #[serde(rename = "totalStockholderEquity")]
    pub(crate) total_stockholder_equity: Option<RawNum>,

    pub(crate) cash: Option<RawNum>,

    #[serde(rename = "longTermDebt")]
    pub(crate) long_term_debt: Option<RawNum>,
}

/* --- cashflow --- */
#[derive(Deserialize)]
pub(crate) struct CashflowHistoryNode {
    #[serde(rename = "cashflowStatements")]
    pub(crate) cashflow_statements: Option<Vec<CashflowRowNode>>,
}

#[derive(Deserialize)]
pub(crate) struct CashflowRowNode {
    #[serde(rename = "endDate")]
    pub(crate) end_date: Option<RawDate>,

    #[serde(rename = "totalCashFromOperatingActivities")]
    pub(crate) total_cash_from_operating_activities: Option<RawNum>,

    #[serde(rename = "capitalExpenditures")]
    pub(crate) capital_expenditures: Option<RawNum>,

    #[serde(rename = "freeCashflow")]
    pub(crate) free_cashflow: Option<RawNum>,

    #[serde(rename = "netIncome")]
    pub(crate) net_income: Option<RawNum>,
}

/* --- earnings --- */
#[derive(Deserialize)]
pub(crate) struct EarningsNode {
    #[serde(rename = "financialsChart")]
    pub(crate) financials_chart: Option<FinancialsChartNode>,
    #[serde(rename = "earningsChart")]
    pub(crate) earnings_chart: Option<EarningsChartNode>,
}

#[derive(Deserialize)]
pub(crate) struct FinancialsChartNode {
    pub(crate) yearly: Option<Vec<FinancialYearNode>>,
    pub(crate) quarterly: Option<Vec<FinancialQuarterNode>>,
}

#[derive(Deserialize)]
pub(crate) struct FinancialYearNode {
    pub(crate) date: Option<i64>,
    pub(crate) revenue: Option<RawNum>,
    pub(crate) earnings: Option<RawNum>,
}

#[derive(Deserialize)]
pub(crate) struct FinancialQuarterNode {
    pub(crate) date: Option<String>,
    pub(crate) revenue: Option<RawNum>,
    pub(crate) earnings: Option<RawNum>,
}

#[derive(Deserialize)]
pub(crate) struct EarningsChartNode {
    pub(crate) quarterly: Option<Vec<EpsQuarterNode>>,
}

#[derive(Deserialize)]
pub(crate) struct EpsQuarterNode {
    pub(crate) date: Option<String>,
    pub(crate) actual: Option<RawNum>,
    pub(crate) estimate: Option<RawNum>,
}

/* --- calendar --- */
#[derive(Deserialize)]
pub(crate) struct CalendarEventsNode {
    pub(crate) earnings: Option<CalendarEarningsNode>,
}

#[derive(Deserialize)]
pub(crate) struct CalendarEarningsNode {
    #[serde(rename = "earningsDate")]
    pub(crate) earnings_date: Option<Vec<RawDate>>,
    #[serde(rename = "exDividendDate")]
    pub(crate) ex_dividend_date: Option<RawDate>,
    #[serde(rename = "dividendDate")]
    pub(crate) dividend_date: Option<RawDate>,
}

/* --- shared small wrappers + helpers --- */

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct RawDate {
    pub(crate) raw: Option<i64>,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct RawNum {
    pub(crate) raw: Option<f64>,
}

pub(crate) fn raw_num(n: RawNum) -> Option<f64> {
    n.raw
}
