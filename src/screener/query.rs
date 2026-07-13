use std::marker::PhantomData;

use paft::Decimal;
use paft::domain::AssetKind;
use serde::Serialize;
use serde_json::{Value, json};

use crate::{YfError, core::yahoo_vocab::parse_yahoo_quote_type};

/// Marker type for predefined screen requests.
#[derive(Debug, Clone, Copy)]
pub struct Predefined;

/// Marker type for equity screen queries.
#[derive(Debug, Clone, Copy)]
pub struct Equity;

/// Marker type for mutual fund screen queries.
#[derive(Debug, Clone, Copy)]
pub struct Fund;

/// Marker type for ETF screen queries.
#[derive(Debug, Clone, Copy)]
pub struct Etf;

/// Equity screener query.
pub type EquityQuery = ScreenerQuery<Equity>;

/// Mutual fund screener query.
pub type FundQuery = ScreenerQuery<Fund>;

/// ETF screener query.
pub type EtfQuery = ScreenerQuery<Etf>;

/// Yahoo quote type used by the screener endpoint.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum YahooQuoteType {
    /// Common stock or equity-like instrument.
    #[serde(rename = "EQUITY")]
    Equity,
    /// Mutual fund.
    #[serde(rename = "MUTUALFUND")]
    MutualFund,
    /// Exchange-traded fund.
    #[serde(rename = "ETF")]
    Etf,
}

impl YahooQuoteType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Equity => "EQUITY",
            Self::MutualFund => "MUTUALFUND",
            Self::Etf => "ETF",
        }
    }

    pub(crate) fn asset_kind(self) -> AssetKind {
        parse_yahoo_quote_type(self.as_str()).expect("typed screener quote type is valid")
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "EQUITY" => Some(Self::Equity),
            "MUTUALFUND" => Some(Self::MutualFund),
            "ETF" => Some(Self::Etf),
            _ => None,
        }
    }
}

/// Number of screener rows to request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScreenerCount(u32);

impl ScreenerCount {
    /// Maximum count Yahoo accepts.
    pub const MAX: u32 = 250;

    pub(crate) const DEFAULT: Self = Self(25);

    /// Builds a validated screener count.
    ///
    /// # Errors
    ///
    /// Returns `YfError::InvalidParams` if the count is zero or greater than
    /// Yahoo's maximum of 250.
    pub fn new(value: u32) -> Result<Self, YfError> {
        if value == 0 || value > Self::MAX {
            return Err(YfError::InvalidParams(format!(
                "screener count must be between 1 and {}",
                Self::MAX
            )));
        }
        Ok(Self(value))
    }

    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

/// Nonnegative result offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultOffset(u32);

impl ResultOffset {
    pub(crate) const ZERO: Self = Self(0);

    /// Builds an offset from an unsigned value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Builds an offset from a signed value.
    ///
    /// # Errors
    ///
    /// Returns `YfError::InvalidParams` for negative values or values too large
    /// for `u32`.
    pub fn try_from_i64(value: i64) -> Result<Self, YfError> {
        let value = u32::try_from(value)
            .map_err(|_| YfError::InvalidParams("screener offset must be nonnegative".into()))?;
        Ok(Self(value))
    }

    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

/// An exact decimal screener filter value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenerNumber(Decimal);

impl ScreenerNumber {
    /// Builds an exact decimal screener number.
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    fn to_value(self) -> Value {
        Value::Number(
            self.0
                .to_string()
                .parse()
                .expect("Decimal always has a valid JSON number representation"),
        )
    }
}

impl From<Decimal> for ScreenerNumber {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<u32> for ScreenerNumber {
    fn from(value: u32) -> Self {
        Self(Decimal::from(value))
    }
}

impl From<i32> for ScreenerNumber {
    fn from(value: i32) -> Self {
        Self(Decimal::from(value))
    }
}

impl From<u64> for ScreenerNumber {
    fn from(value: u64) -> Self {
        Self(Decimal::from(value))
    }
}

impl From<i64> for ScreenerNumber {
    fn from(value: i64) -> Self {
        Self(Decimal::from(value))
    }
}

/// Percent-point filter value.
///
/// Yahoo screener percent fields expect `3` for 3%, not `0.03`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PercentPoints(ScreenerNumber);

impl PercentPoints {
    /// Builds an exact decimal percent-point value.
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(ScreenerNumber::new(value))
    }
}

/// Sort direction for custom screener POST bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Ascending sort order.
    Asc,
    /// Descending sort order.
    Desc,
}

impl SortDirection {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

/// Yahoo screener region code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// United States.
    Us,
}

impl Region {
    const fn as_query_value(self) -> &'static str {
        match self {
            Self::Us => "us",
        }
    }
}

/// Yahoo exchange code used by screener filters.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum YahooExchangeCode {
    /// NASDAQ Global Select.
    #[serde(rename = "NMS")]
    Nms,
    /// NASDAQ Global Market.
    #[serde(rename = "NGM")]
    Ngm,
    /// NASDAQ Capital Market.
    #[serde(rename = "NCM")]
    Ncm,
    /// NYSE.
    #[serde(rename = "NYQ")]
    Nyq,
    /// NASDAQ mutual fund exchange code.
    #[serde(rename = "NAS")]
    Nas,
    /// NYSE American.
    #[serde(rename = "ASE")]
    Ase,
    /// BATS.
    #[serde(rename = "BTS")]
    Bts,
    /// NYSE Arca.
    #[serde(rename = "PCX")]
    Pcx,
    /// OTC Pink.
    #[serde(rename = "PNK")]
    Pnk,
    /// OTCQB.
    #[serde(rename = "OQB")]
    Oqb,
    /// OTCQX.
    #[serde(rename = "OQX")]
    Oqx,
    /// Other OTC market.
    #[serde(rename = "OEM")]
    Oem,
    /// Yahoo YHD venue code.
    #[serde(rename = "YHD")]
    Yhd,
    /// Yahoo CXI venue code.
    #[serde(rename = "CXI")]
    Cxi,
    /// Yahoo NAE venue code.
    #[serde(rename = "NAE")]
    Nae,
    /// OTC Global Market.
    #[serde(rename = "OGM")]
    Ogm,
    /// WCB venue code.
    #[serde(rename = "WCB")]
    Wcb,
}

impl YahooExchangeCode {
    /// Yahoo wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Nms => "NMS",
            Self::Ngm => "NGM",
            Self::Ncm => "NCM",
            Self::Nyq => "NYQ",
            Self::Nas => "NAS",
            Self::Ase => "ASE",
            Self::Bts => "BTS",
            Self::Pcx => "PCX",
            Self::Pnk => "PNK",
            Self::Oqb => "OQB",
            Self::Oqx => "OQX",
            Self::Oem => "OEM",
            Self::Yhd => "YHD",
            Self::Cxi => "CXI",
            Self::Nae => "NAE",
            Self::Ogm => "OGM",
            Self::Wcb => "WCB",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "NMS" => Some(Self::Nms),
            "NGM" => Some(Self::Ngm),
            "NCM" => Some(Self::Ncm),
            "NYQ" => Some(Self::Nyq),
            "NAS" => Some(Self::Nas),
            "ASE" => Some(Self::Ase),
            "BTS" => Some(Self::Bts),
            "PCX" => Some(Self::Pcx),
            "PNK" => Some(Self::Pnk),
            "OQB" => Some(Self::Oqb),
            "OQX" => Some(Self::Oqx),
            "OEM" => Some(Self::Oem),
            "YHD" => Some(Self::Yhd),
            "CXI" => Some(Self::Cxi),
            "NAE" => Some(Self::Nae),
            "OGM" => Some(Self::Ogm),
            "WCB" => Some(Self::Wcb),
            _ => None,
        }
    }
}

/// Equity sector values supported by Yahoo screeners.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquitySector {
    /// Basic materials.
    BasicMaterials,
    /// Industrials.
    Industrials,
    /// Communication services.
    CommunicationServices,
    /// Healthcare.
    Healthcare,
    /// Real estate.
    RealEstate,
    /// Technology.
    Technology,
    /// Energy.
    Energy,
    /// Utilities.
    Utilities,
    /// Financial services.
    FinancialServices,
    /// Consumer defensive.
    ConsumerDefensive,
    /// Consumer cyclical.
    ConsumerCyclical,
}

impl EquitySector {
    const fn as_str(self) -> &'static str {
        match self {
            Self::BasicMaterials => "Basic Materials",
            Self::Industrials => "Industrials",
            Self::CommunicationServices => "Communication Services",
            Self::Healthcare => "Healthcare",
            Self::RealEstate => "Real Estate",
            Self::Technology => "Technology",
            Self::Energy => "Energy",
            Self::Utilities => "Utilities",
            Self::FinancialServices => "Financial Services",
            Self::ConsumerDefensive => "Consumer Defensive",
            Self::ConsumerCyclical => "Consumer Cyclical",
        }
    }
}

/// Mutual fund category values currently supported by the typed API.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FundCategory {
    /// Foreign large value.
    ForeignLargeValue,
    /// Foreign large blend.
    ForeignLargeBlend,
    /// Foreign large growth.
    ForeignLargeGrowth,
    /// Foreign small/mid growth.
    ForeignSmallMidGrowth,
    /// Foreign small/mid blend.
    ForeignSmallMidBlend,
    /// Foreign small/mid value.
    ForeignSmallMidValue,
    /// High yield bond.
    HighYieldBond,
    /// Large blend.
    LargeBlend,
    /// Large growth.
    LargeGrowth,
    /// Mid-cap growth.
    MidCapGrowth,
}

impl FundCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ForeignLargeValue => "Foreign Large Value",
            Self::ForeignLargeBlend => "Foreign Large Blend",
            Self::ForeignLargeGrowth => "Foreign Large Growth",
            Self::ForeignSmallMidGrowth => "Foreign Small/Mid Growth",
            Self::ForeignSmallMidBlend => "Foreign Small/Mid Blend",
            Self::ForeignSmallMidValue => "Foreign Small/Mid Value",
            Self::HighYieldBond => "High Yield Bond",
            Self::LargeBlend => "Large Blend",
            Self::LargeGrowth => "Large Growth",
            Self::MidCapGrowth => "Mid-Cap Growth",
        }
    }
}

/// ETF category values currently supported by the typed API.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EtfCategory {
    /// Technology.
    Technology,
    /// Corporate bond.
    CorporateBond,
    /// Emerging markets bond.
    EmergingMarketsBond,
    /// Emerging-markets local-currency bond.
    EmergingMarketsLocalCurrencyBond,
    /// High yield bond.
    HighYieldBond,
    /// Intermediate-term bond.
    IntermediateTermBond,
    /// Long-term bond.
    LongTermBond,
    /// Inflation-protected bond.
    InflationProtectedBond,
    /// Multisector bond.
    MultisectorBond,
    /// Nontraditional bond.
    NontraditionalBond,
    /// Short-term bond.
    ShortTermBond,
    /// Ultrashort bond.
    UltrashortBond,
    /// World bond.
    WorldBond,
}

impl EtfCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Technology => "Technology",
            Self::CorporateBond => "Corporate Bond",
            Self::EmergingMarketsBond => "Emerging Markets Bond",
            Self::EmergingMarketsLocalCurrencyBond => "Emerging-Markets Local-Currency Bond",
            Self::HighYieldBond => "High Yield Bond",
            Self::IntermediateTermBond => "Intermediate-Term Bond",
            Self::LongTermBond => "Long-Term Bond",
            Self::InflationProtectedBond => "Inflation-Protected Bond",
            Self::MultisectorBond => "Multisector Bond",
            Self::NontraditionalBond => "Nontraditional Bond",
            Self::ShortTermBond => "Short-Term Bond",
            Self::UltrashortBond => "Ultrashort Bond",
            Self::WorldBond => "World Bond",
        }
    }
}

/// Yahoo fund performance or risk rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rating {
    /// One-star rating.
    One,
    /// Two-star rating.
    Two,
    /// Three-star rating.
    Three,
    /// Four-star rating.
    Four,
    /// Five-star rating.
    Five,
}

impl Rating {
    const fn value(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::Five => 5,
        }
    }
}

mod sealed {
    pub trait Sealed {}
}

/// Sealed trait for values that can be used with typed enum screener fields.
pub trait ScreenerValue: Copy + sealed::Sealed {
    /// Converts this value into Yahoo's screener wire representation.
    #[doc(hidden)]
    fn to_wire_value(self) -> Value;
}

macro_rules! impl_screener_value {
    ($type:ty, $body:expr) => {
        impl sealed::Sealed for $type {}

        impl ScreenerValue for $type {
            fn to_wire_value(self) -> Value {
                $body(self)
            }
        }
    };
}

impl_screener_value!(ScreenerNumber, ScreenerNumber::to_value);
impl_screener_value!(PercentPoints, |value: PercentPoints| value.0.to_value());
impl_screener_value!(Region, |value: Region| {
    Value::String(value.as_query_value().to_string())
});
impl_screener_value!(YahooExchangeCode, |value: YahooExchangeCode| {
    Value::String(value.code().to_string())
});
impl_screener_value!(EquitySector, |value: EquitySector| {
    Value::String(value.as_str().to_string())
});
impl_screener_value!(FundCategory, |value: FundCategory| {
    Value::String(value.as_str().to_string())
});
impl_screener_value!(EtfCategory, |value: EtfCategory| {
    Value::String(value.as_str().to_string())
});
impl_screener_value!(Rating, |value: Rating| {
    Value::Number(serde_json::Number::from(value.value()))
});

/// Typed numeric screener field.
#[derive(Debug, Clone, Copy)]
pub struct NumericField<U> {
    key: &'static str,
    marker: PhantomData<U>,
}

impl<U> NumericField<U> {
    pub(crate) const fn new(key: &'static str) -> Self {
        Self {
            key,
            marker: PhantomData,
        }
    }

    /// Builds an equality condition.
    #[must_use]
    pub fn eq<N: Into<ScreenerNumber>>(self, value: N) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("EQ", self.key, value.into().to_value())
    }

    /// Builds a one-of condition.
    ///
    /// # Errors
    ///
    /// Returns `YfError::InvalidParams` if no values are provided.
    pub fn one_of<I, N>(self, values: I) -> Result<ScreenerQuery<U>, YfError>
    where
        I: IntoIterator<Item = N>,
        N: Into<ScreenerNumber>,
    {
        ScreenerQuery::one_of_values(self.key, values.into_iter().map(|v| v.into().to_value()))
    }

    /// Builds a greater-than condition.
    #[must_use]
    pub fn gt<N: Into<ScreenerNumber>>(self, value: N) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("GT", self.key, value.into().to_value())
    }

    /// Builds a greater-than-or-equal condition.
    #[must_use]
    pub fn gte<N: Into<ScreenerNumber>>(self, value: N) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("GTE", self.key, value.into().to_value())
    }

    /// Builds a less-than condition.
    #[must_use]
    pub fn lt<N: Into<ScreenerNumber>>(self, value: N) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("LT", self.key, value.into().to_value())
    }

    /// Builds a less-than-or-equal condition.
    #[must_use]
    pub fn lte<N: Into<ScreenerNumber>>(self, value: N) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("LTE", self.key, value.into().to_value())
    }

    /// Builds a between condition.
    #[must_use]
    pub fn between<N1, N2>(self, lower: N1, upper: N2) -> ScreenerQuery<U>
    where
        N1: Into<ScreenerNumber>,
        N2: Into<ScreenerNumber>,
    {
        ScreenerQuery::comparison(
            "BTWN",
            self.key,
            json!([lower.into().to_value(), upper.into().to_value()]),
        )
    }
}

/// Typed percent-point screener field.
#[derive(Debug, Clone, Copy)]
pub struct PercentField<U> {
    key: &'static str,
    marker: PhantomData<U>,
}

impl<U> PercentField<U> {
    pub(crate) const fn new(key: &'static str) -> Self {
        Self {
            key,
            marker: PhantomData,
        }
    }

    /// Builds a greater-than condition.
    #[must_use]
    pub fn gt(self, value: PercentPoints) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("GT", self.key, value.to_wire_value())
    }

    /// Builds a greater-than-or-equal condition.
    #[must_use]
    pub fn gte(self, value: PercentPoints) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("GTE", self.key, value.to_wire_value())
    }

    /// Builds a less-than condition.
    #[must_use]
    pub fn lt(self, value: PercentPoints) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("LT", self.key, value.to_wire_value())
    }

    /// Builds a less-than-or-equal condition.
    #[must_use]
    pub fn lte(self, value: PercentPoints) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("LTE", self.key, value.to_wire_value())
    }
}

/// Typed enum-like screener field.
#[derive(Debug, Clone, Copy)]
pub struct EnumField<U, V> {
    key: &'static str,
    marker: PhantomData<(U, V)>,
}

impl<U, V> EnumField<U, V>
where
    V: ScreenerValue,
{
    pub(crate) const fn new(key: &'static str) -> Self {
        Self {
            key,
            marker: PhantomData,
        }
    }

    /// Builds an equality condition.
    #[must_use]
    pub fn eq(self, value: V) -> ScreenerQuery<U> {
        ScreenerQuery::comparison("EQ", self.key, value.to_wire_value())
    }

    /// Builds a one-of condition.
    ///
    /// # Errors
    ///
    /// Returns `YfError::InvalidParams` if no values are provided.
    pub fn one_of<I>(self, values: I) -> Result<ScreenerQuery<U>, YfError>
    where
        I: IntoIterator<Item = V>,
    {
        ScreenerQuery::one_of_values(
            self.key,
            values.into_iter().map(ScreenerValue::to_wire_value),
        )
    }
}

/// Typed sort field.
#[derive(Debug, Clone, Copy)]
pub struct SortField<U> {
    key: &'static str,
    marker: PhantomData<U>,
}

impl<U> SortField<U> {
    pub(crate) const fn new(key: &'static str) -> Self {
        Self {
            key,
            marker: PhantomData,
        }
    }

    pub(crate) const fn key(self) -> &'static str {
        self.key
    }
}

#[derive(Debug, Clone)]
enum QueryNode {
    Comparison {
        operator: &'static str,
        field: &'static str,
        values: Vec<Value>,
    },
    Logical {
        operator: &'static str,
        operands: Vec<Self>,
    },
}

impl QueryNode {
    fn to_wire_value(&self) -> Value {
        match self {
            Self::Comparison {
                operator,
                field,
                values,
            } => {
                let mut operands = Vec::with_capacity(values.len() + 1);
                operands.push(Value::String((*field).to_string()));
                operands.extend(values.iter().cloned());
                json!({
                    "operator": *operator,
                    "operands": operands,
                })
            }
            Self::Logical { operator, operands } => json!({
                "operator": *operator,
                "operands": operands.iter().map(Self::to_wire_value).collect::<Vec<_>>(),
            }),
        }
    }
}

/// Strongly typed Yahoo screener query.
#[derive(Debug, Clone)]
pub struct ScreenerQuery<U> {
    node: QueryNode,
    marker: PhantomData<U>,
}

impl<U> ScreenerQuery<U> {
    fn comparison(operator: &'static str, field: &'static str, value: Value) -> Self {
        let values = match (operator, value) {
            ("BTWN", Value::Array(values)) => values,
            (_, value) => vec![value],
        };

        Self {
            node: QueryNode::Comparison {
                operator,
                field,
                values,
            },
            marker: PhantomData,
        }
    }

    fn one_of_values<I>(field: &'static str, values: I) -> Result<Self, YfError>
    where
        I: IntoIterator<Item = Value>,
    {
        let operands = values
            .into_iter()
            .map(|value| QueryNode::Comparison {
                operator: "EQ",
                field,
                values: vec![value],
            })
            .collect::<Vec<_>>();

        if operands.is_empty() {
            return Err(YfError::InvalidParams(
                "one_of requires at least one value".into(),
            ));
        }

        Ok(Self {
            node: QueryNode::Logical {
                operator: "OR",
                operands,
            },
            marker: PhantomData,
        })
    }

    /// Combines queries with logical AND.
    ///
    /// # Errors
    ///
    /// Returns `YfError::InvalidParams` unless at least two child queries are
    /// provided.
    pub fn and<I>(queries: I) -> Result<Self, YfError>
    where
        I: IntoIterator<Item = Self>,
    {
        Self::logical("AND", queries)
    }

    /// Combines queries with logical OR.
    ///
    /// # Errors
    ///
    /// Returns `YfError::InvalidParams` unless at least two child queries are
    /// provided.
    pub fn or<I>(queries: I) -> Result<Self, YfError>
    where
        I: IntoIterator<Item = Self>,
    {
        Self::logical("OR", queries)
    }

    fn logical<I>(operator: &'static str, queries: I) -> Result<Self, YfError>
    where
        I: IntoIterator<Item = Self>,
    {
        let operands = queries.into_iter().map(|q| q.node).collect::<Vec<_>>();

        if operands.len() <= 1 {
            return Err(YfError::InvalidParams(format!(
                "{operator} requires at least two operands"
            )));
        }

        Ok(Self {
            node: QueryNode::Logical { operator, operands },
            marker: PhantomData,
        })
    }

    pub(crate) fn into_wire_value(self) -> Value {
        self.node.to_wire_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screener::equity_fields;

    #[test]
    fn one_of_expands_to_or_of_eq_filters() {
        let query = equity_fields::EXCHANGE
            .one_of([YahooExchangeCode::Nms, YahooExchangeCode::Nyq])
            .unwrap();

        assert_eq!(
            query.into_wire_value(),
            json!({
                "operator": "OR",
                "operands": [
                    {"operator": "EQ", "operands": ["exchange", "NMS"]},
                    {"operator": "EQ", "operands": ["exchange", "NYQ"]}
                ]
            })
        );
    }

    #[test]
    fn and_rejects_empty_or_single_operand() {
        assert!(EquityQuery::and(Vec::<EquityQuery>::new()).is_err());
        assert!(EquityQuery::and([equity_fields::INTRADAY_PRICE.gt(1)]).is_err());
    }

    #[test]
    fn screener_number_serializes_exact_decimal() {
        let query = equity_fields::INTRADAY_PRICE.gt(ScreenerNumber::new(Decimal::new(125, 1)));

        assert_eq!(
            query.into_wire_value(),
            json!({
                "operator": "GT",
                "operands": ["intradayprice", 12.5]
            })
        );
    }

    #[test]
    fn bounded_values_reject_invalid_inputs() {
        assert!(ScreenerCount::new(0).is_err());
        assert!(ScreenerCount::new(251).is_err());
        assert!(ResultOffset::try_from_i64(-1).is_err());
    }

    #[test]
    fn screener_number_preserves_more_than_f64_precision() {
        let exact: Decimal = "1234567890.123456789012345678"
            .parse()
            .expect("valid decimal");
        let query = equity_fields::INTRADAY_PRICE.gt(ScreenerNumber::new(exact));

        assert_eq!(
            query.into_wire_value()["operands"][1].to_string(),
            "1234567890.123456789012345678"
        );
    }
}
