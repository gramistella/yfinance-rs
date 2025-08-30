use crate::core::wire::{RawDate, RawNum, RawNumU64};
use serde::Deserialize;

/* ---------------- Serde mapping (only what we need) ---------------- */

#[derive(Deserialize)]
pub struct V10Result {
    /* income */
    #[serde(rename = "incomeStatementHistory")]
    pub(crate) income_statement_history: Option<IncomeHistoryNode>,
    #[serde(rename = "incomeStatementHistoryQuarterly")]
    pub(crate) income_statement_history_quarterly: Option<IncomeHistoryNode>,

    /* earnings + calendar */
    pub(crate) earnings: Option<EarningsNode>,
    #[serde(rename = "calendarEvents")]
    pub(crate) calendar_events: Option<CalendarEventsNode>,
}

/* --- income --- */
#[derive(Deserialize)]
pub struct IncomeHistoryNode {
    #[serde(rename = "incomeStatementHistory")]
    pub(crate) income_statement_history: Option<Vec<IncomeRowNode>>,
}

#[derive(Deserialize)]
pub struct IncomeRowNode {
    #[serde(rename = "endDate")]
    pub(crate) end_date: Option<RawDate>,
    #[serde(rename = "totalRevenue")]
    pub(crate) total_revenue: Option<RawNum<f64>>,
    #[serde(rename = "grossProfit")]
    pub(crate) gross_profit: Option<RawNum<f64>>,
    #[serde(rename = "operatingIncome")]
    pub(crate) operating_income: Option<RawNum<f64>>,
    #[serde(rename = "netIncome")]
    pub(crate) net_income: Option<RawNum<f64>>,
}

/* --- earnings --- */
#[derive(Deserialize)]
pub struct EarningsNode {
    #[serde(rename = "financialsChart")]
    pub(crate) financials_chart: Option<FinancialsChartNode>,
    #[serde(rename = "earningsChart")]
    pub(crate) earnings_chart: Option<EarningsChartNode>,
}

#[derive(Deserialize)]
pub struct FinancialsChartNode {
    pub(crate) yearly: Option<Vec<FinancialYearNode>>,
    pub(crate) quarterly: Option<Vec<FinancialQuarterNode>>,
}

#[derive(Deserialize)]
pub struct FinancialYearNode {
    pub(crate) date: Option<i64>,
    pub(crate) revenue: Option<RawNum<f64>>,
    pub(crate) earnings: Option<RawNum<f64>>,
}

#[derive(Deserialize)]
pub struct FinancialQuarterNode {
    pub(crate) date: Option<String>,
    pub(crate) revenue: Option<RawNum<f64>>,
    pub(crate) earnings: Option<RawNum<f64>>,
}

#[derive(Deserialize)]
pub struct EarningsChartNode {
    pub(crate) quarterly: Option<Vec<EpsQuarterNode>>,
}

#[derive(Deserialize)]
pub struct EpsQuarterNode {
    pub(crate) date: Option<String>,
    pub(crate) actual: Option<RawNum<f64>>,
    pub(crate) estimate: Option<RawNum<f64>>,
}

/* --- calendar --- */
#[derive(Deserialize)]
pub struct CalendarEventsNode {
    pub(crate) earnings: Option<CalendarEarningsNode>,
}

#[derive(Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct CalendarEarningsNode {
    #[serde(rename = "earningsDate")]
    pub(crate) earnings_date: Option<Vec<RawDate>>,
    #[serde(rename = "exDividendDate")]
    pub(crate) ex_dividend_date: Option<RawDate>,
    #[serde(rename = "dividendDate")]
    pub(crate) dividend_date: Option<RawDate>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeseriesEnvelope {
    pub(crate) timeseries: Option<TimeseriesResult>,
}

#[derive(Deserialize)]
pub struct TimeseriesResult {
    pub(crate) result: Option<Vec<TimeseriesData>>,
}

#[derive(Deserialize)]
pub struct TimeseriesData {
    pub(crate) timestamp: Option<Vec<i64>>,
    #[allow(dead_code)]
    meta: serde_json::Value,
    #[serde(flatten)]
    pub(crate) values: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
pub struct TimeseriesValue {
    #[serde(rename = "reportedValue")]
    pub(crate) reported_value: Option<RawNumU64>,
}
