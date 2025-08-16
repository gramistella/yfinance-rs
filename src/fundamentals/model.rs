use serde::Serialize;

/// Common numeric type for money-like values.
/// Yahoo mixes ints/floats; we normalize to f64.
pub type Num = f64;

/// Statement row for Income Statement (annual or quarterly).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IncomeStatementRow {
    pub period_end: i64,                // unix ts (UTC) of period end
    pub total_revenue: Option<Num>,
    pub gross_profit: Option<Num>,
    pub operating_income: Option<Num>,
    pub net_income: Option<Num>,
}

/// Statement row for Balance Sheet (annual or quarterly).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BalanceSheetRow {
    pub period_end: i64,
    pub total_assets: Option<Num>,
    pub total_liabilities: Option<Num>,
    pub total_equity: Option<Num>,
    pub cash: Option<Num>,
    pub long_term_debt: Option<Num>,
}

/// Statement row for Cashflow (annual or quarterly).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CashflowRow {
    pub period_end: i64,
    pub operating_cashflow: Option<Num>,
    pub capital_expenditures: Option<Num>,
    /// If Yahoo doesn't provide, we compute as operating - capex when possible.
    pub free_cash_flow: Option<Num>,
    pub net_income: Option<Num>,
}

/// Earnings summary (financialsChart + earningsChart)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Earnings {
    pub yearly: Vec<EarningsYear>,
    pub quarterly: Vec<EarningsQuarter>,
    pub quarterly_eps: Vec<EarningsQuarterEps>, // from earningsChart.quarterly
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EarningsYear {
    pub year: i32,
    pub revenue: Option<Num>,
    pub earnings: Option<Num>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EarningsQuarter {
    /// e.g. "2024Q1" or "2024-04"
    pub period: String,
    pub revenue: Option<Num>,
    pub earnings: Option<Num>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EarningsQuarterEps {
    /// e.g. "2024Q1" or "2024-04"
    pub period: String,
    pub actual: Option<Num>,
    pub estimate: Option<Num>,
}

/// Company “calendar” fields (from calendarEvents.earnings)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Calendar {
    pub earnings_dates: Vec<i64>,        // usually 1–2 dates
    pub ex_dividend_date: Option<i64>,
    pub dividend_date: Option<i64>,
}
