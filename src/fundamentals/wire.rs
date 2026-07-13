use crate::core::wire::{BorrowedWireValue, RawDate, RawDecimal, RawNumU64, WireValue};
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};
use serde_json::value::RawValue;
use std::{collections::HashMap, fmt, marker::PhantomData};

/* ---------------- Serde mapping (only what we need) ---------------- */

#[derive(Deserialize)]
pub struct V10Result<'a> {
    /* income */
    #[allow(dead_code)]
    #[serde(rename = "incomeStatementHistory")]
    pub(crate) income_statement_history: Option<IncomeHistoryNode>,
    #[allow(dead_code)]
    #[serde(rename = "incomeStatementHistoryQuarterly")]
    pub(crate) income_statement_history_quarterly: Option<IncomeHistoryNode>,

    /* earnings + calendar */
    #[serde(default, borrow)]
    pub(crate) earnings: BorrowedWireValue<'a, EarningsNode<'a>>,
    #[serde(rename = "calendarEvents", default, borrow)]
    pub(crate) calendar_events: BorrowedWireValue<'a, CalendarEventsNode<'a>>,
}

/* --- income --- */
#[derive(Deserialize)]
pub struct IncomeHistoryNode {
    #[allow(dead_code)]
    #[serde(rename = "incomeStatementHistory")]
    pub(crate) income_statement_history: Option<Vec<IncomeRowNode>>,
}

#[derive(Deserialize)]
pub struct IncomeRowNode {
    #[allow(dead_code)]
    #[serde(rename = "endDate")]
    #[serde(default)]
    pub(crate) end_date: WireValue<RawDate>,
    #[allow(dead_code)]
    #[serde(rename = "totalRevenue")]
    #[serde(default)]
    pub(crate) total_revenue: WireValue<RawDecimal>,
    #[allow(dead_code)]
    #[serde(rename = "grossProfit")]
    #[serde(default)]
    pub(crate) gross_profit: WireValue<RawDecimal>,
    #[allow(dead_code)]
    #[serde(rename = "operatingIncome")]
    #[serde(default)]
    pub(crate) operating_income: WireValue<RawDecimal>,
    #[allow(dead_code)]
    #[serde(rename = "netIncome")]
    #[serde(default)]
    pub(crate) net_income: WireValue<RawDecimal>,
}

/* --- earnings --- */
#[derive(Deserialize)]
pub struct EarningsNode<'a> {
    #[serde(rename = "financialCurrency")]
    #[serde(default)]
    pub(crate) financial_currency: WireValue<String>,
    #[serde(rename = "financialsChart", default, borrow)]
    pub(crate) financials_chart: BorrowedWireValue<'a, FinancialsChartNode<'a>>,
    #[serde(rename = "earningsChart", default, borrow)]
    pub(crate) earnings_chart: BorrowedWireValue<'a, EarningsChartNode<'a>>,
}

#[derive(Deserialize)]
pub struct FinancialsChartNode<'a> {
    #[serde(default, borrow)]
    pub(crate) yearly: BorrowedWireValue<'a, Vec<FinancialYearNode>>,
    #[serde(default, borrow)]
    pub(crate) quarterly: BorrowedWireValue<'a, Vec<FinancialQuarterNode>>,
}

#[derive(Deserialize)]
pub struct FinancialYearNode {
    #[serde(default)]
    pub(crate) date: WireValue<i64>,
    #[serde(default)]
    pub(crate) revenue: WireValue<RawDecimal>,
    #[serde(default)]
    pub(crate) earnings: WireValue<RawDecimal>,
}

#[derive(Deserialize)]
pub struct FinancialQuarterNode {
    #[serde(default)]
    pub(crate) date: WireValue<String>,
    #[serde(default)]
    pub(crate) revenue: WireValue<RawDecimal>,
    #[serde(default)]
    pub(crate) earnings: WireValue<RawDecimal>,
}

#[derive(Deserialize)]
pub struct EarningsChartNode<'a> {
    #[serde(default, borrow)]
    pub(crate) quarterly: BorrowedWireValue<'a, Vec<EpsQuarterNode>>,
}

#[derive(Deserialize)]
pub struct EpsQuarterNode {
    #[serde(default)]
    pub(crate) date: WireValue<String>,
    #[serde(default)]
    pub(crate) actual: WireValue<RawDecimal>,
    #[serde(default)]
    pub(crate) estimate: WireValue<RawDecimal>,
}

/* --- calendar --- */
#[derive(Deserialize)]
pub struct CalendarEventsNode<'a> {
    #[serde(default, borrow)]
    pub(crate) earnings: BorrowedWireValue<'a, CalendarEarningsNode<'a>>,
    #[serde(rename = "exDividendDate")]
    #[serde(default)]
    pub(crate) ex_dividend_date: WireValue<RawDate>,
    #[serde(rename = "dividendDate")]
    #[serde(default)]
    pub(crate) dividend_date: WireValue<RawDate>,
}

#[derive(Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct CalendarEarningsNode<'a> {
    #[serde(rename = "earningsDate", default, borrow)]
    pub(crate) earnings_date: BorrowedWireValue<'a, Vec<RawDate>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeseriesEnvelope<'a> {
    #[serde(borrow)]
    pub(crate) timeseries: Option<TimeseriesResult<'a>>,
    #[serde(borrow)]
    pub(crate) finance: Option<TimeseriesErrorNode<'a>>,
}

#[derive(Deserialize)]
pub struct TimeseriesResult<'a> {
    #[serde(borrow)]
    pub(crate) result: Option<Vec<TimeseriesData<'a>>>,
    #[serde(borrow)]
    pub(crate) error: Option<&'a RawValue>,
}

#[derive(Deserialize)]
pub struct TimeseriesErrorNode<'a> {
    #[serde(borrow)]
    pub(crate) error: Option<&'a RawValue>,
}

pub struct TimeseriesData<'a> {
    pub(crate) timestamp: Option<Vec<i64>>,
    #[allow(dead_code)]
    pub(crate) meta: Option<&'a RawValue>,
    pub(crate) values: HashMap<String, &'a RawValue>,
}

impl<'de: 'a, 'a> Deserialize<'de> for TimeseriesData<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(TimeseriesDataVisitor(PhantomData))
    }
}

struct TimeseriesDataVisitor<'a>(PhantomData<&'a ()>);

impl<'de: 'a, 'a> Visitor<'de> for TimeseriesDataVisitor<'a> {
    type Value = TimeseriesData<'a>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a Yahoo fundamentals timeseries item")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut timestamp = None;
        let mut meta = None;
        let mut values = HashMap::with_capacity(map.size_hint().unwrap_or(0).saturating_sub(2));

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "timestamp" => timestamp = map.next_value()?,
                "meta" => meta = map.next_value()?,
                _ => {
                    values.insert(key, map.next_value()?);
                }
            }
        }

        Ok(TimeseriesData {
            timestamp,
            meta,
            values,
        })
    }
}

#[derive(Deserialize)]
pub struct TimeseriesValue {
    #[serde(rename = "reportedValue")]
    #[serde(default)]
    pub(crate) reported_value: WireValue<RawNumU64>,
}
