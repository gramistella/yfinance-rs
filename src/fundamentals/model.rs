use serde::Serialize;

#[cfg(feature = "dataframe")]
use borsa_macros::ToDataFrame;

#[cfg(feature = "dataframe")]
use crate::core::dataframe::ToDataFrame;

/// A common numeric type for financial values, normalized to `f64`.
pub type Num = f64;

/// A row from an income statement (annual or quarterly).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct IncomeStatementRow {
    /// The Unix timestamp (in seconds) marking the end of the financial period.
    pub period_end: i64,
    /// The total revenue for the period.
    pub total_revenue: Option<f64>,
    /// The gross profit for the period.
    pub gross_profit: Option<f64>,
    /// The operating income for the period.
    pub operating_income: Option<f64>,
    /// The net income for the period.
    pub net_income: Option<f64>,
}

/// A row from a balance sheet (annual or quarterly).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct BalanceSheetRow {
    /// The Unix timestamp (in seconds) marking the end of the financial period.
    pub period_end: i64,
    /// The total assets of the company.
    pub total_assets: Option<f64>,
    /// The total liabilities of the company.
    pub total_liabilities: Option<f64>,
    /// The total stockholder equity.
    pub total_equity: Option<f64>,
    /// The amount of cash and cash equivalents.
    pub cash: Option<f64>,
    /// The total long-term debt.
    pub long_term_debt: Option<f64>,
    /// The number of shares outstanding.
    pub shares_outstanding: Option<u64>,
}

/// A row from a cash flow statement (annual or quarterly).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct CashflowRow {
    /// The Unix timestamp (in seconds) marking the end of the financial period.
    pub period_end: i64,
    /// The total cash flow from operating activities.
    pub operating_cashflow: Option<f64>,
    /// Capital expenditures for the period.
    pub capital_expenditures: Option<f64>,
    /// Free cash flow for the period.
    ///
    /// If Yahoo doesn't provide this, it's calculated as `operating_cashflow - capital_expenditures`.
    pub free_cash_flow: Option<f64>,
    /// The net income for the period.
    pub net_income: Option<f64>,
}

/// A summary of historical and estimated earnings data.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Earnings {
    /// A history of annual revenue and earnings.
    pub yearly: Vec<EarningsYear>,
    /// A history of quarterly revenue and earnings.
    pub quarterly: Vec<EarningsQuarter>,
    /// A history of quarterly EPS (Earnings Per Share), including actual and estimated values.
    pub quarterly_eps: Vec<EarningsQuarterEps>,
}

/// Annual revenue and earnings data for a single year.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct EarningsYear {
    /// The calendar year.
    pub year: i32,
    /// The total revenue for the year.
    pub revenue: Option<f64>,
    /// The net earnings for the year.
    pub earnings: Option<f64>,
}

/// Quarterly revenue and earnings data for a single quarter.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct EarningsQuarter {
    /// The quarter identifier (e.g., "2Q2024").
    pub period: String,
    /// The total revenue for the quarter.
    pub revenue: Option<f64>,
    /// The net earnings for the quarter.
    pub earnings: Option<f64>,
}

/// Quarterly EPS (Earnings Per Share) data, including actual and estimated values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct EarningsQuarterEps {
    /// The quarter identifier (e.g., "2Q2024").
    pub period: String,
    /// The actual reported EPS for the quarter.
    pub actual: Option<f64>,
    /// The consensus analyst estimate for EPS for the quarter.
    pub estimate: Option<f64>,
}

/// Corporate calendar events, like earnings and dividend dates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct Calendar {
    /// A list of upcoming earnings announcement dates as Unix timestamps.
    pub earnings_dates: Vec<i64>,
    /// The ex-dividend date as a Unix timestamp.
    pub ex_dividend_date: Option<i64>,
    /// The dividend payment date as a Unix timestamp.
    pub dividend_date: Option<i64>,
}

/// Represents a single data point in a time series of shares outstanding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct ShareCount {
    /// The timestamp for the data point.
    pub date: i64,
    /// The number of shares outstanding.
    pub shares: u64,
}
