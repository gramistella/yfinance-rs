use crate::core::wire::{JsonDecimal, WireValue};
use paft::Decimal;
use serde::Deserialize;
use serde::Deserializer;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct ChartEnvelope {
    pub(crate) chart: Option<ChartNode>,
}

#[derive(Deserialize)]
pub struct ChartNode {
    pub(crate) result: Option<Vec<ChartResult>>,
    pub(crate) error: Option<ChartError>,
}

#[derive(Deserialize)]
pub struct ChartError {
    pub(crate) code: String,
    pub(crate) description: String,
}

#[derive(Deserialize)]
pub struct ChartResult {
    #[serde(default)]
    pub(crate) meta: Option<MetaNode>,
    #[serde(default)]
    pub(crate) timestamp: Option<Vec<i64>>,
    pub(crate) indicators: Indicators,
    #[serde(default)]
    pub(crate) events: Option<Events>,
}

#[derive(Deserialize, Clone, Default)]
pub struct MetaNode {
    #[serde(default)]
    pub(crate) symbol: Option<String>,
    #[serde(default, rename = "instrumentType")]
    pub(crate) instrument_type: Option<String>,
    #[serde(default, rename = "exchangeName")]
    pub(crate) exchange_name: Option<String>,
    #[serde(default, rename = "fullExchangeName")]
    pub(crate) full_exchange_name: Option<String>,
    #[serde(default)]
    pub(crate) timezone: Option<String>,
    #[serde(default, rename = "exchangeTimezoneName")]
    pub(crate) exchange_timezone_name: Option<String>,
    #[serde(default)]
    pub(crate) gmtoffset: Option<i64>,
    #[serde(default)]
    pub(crate) currency: Option<String>,
    #[serde(default, rename = "priceHint")]
    pub(crate) price_hint: Option<i64>,
    #[serde(default, rename = "dataGranularity")]
    pub(crate) data_granularity: Option<String>,
}

#[derive(Deserialize)]
pub struct Indicators {
    #[serde(default)]
    pub(crate) quote: Vec<QuoteBlock>,
    #[serde(default)]
    pub(crate) adjclose: Vec<AdjCloseBlock>,
}

#[derive(Deserialize, Clone)]
pub struct QuoteBlock {
    #[serde(default)]
    pub(crate) open: Vec<WireValue<JsonDecimal>>,
    #[serde(default)]
    pub(crate) high: Vec<WireValue<JsonDecimal>>,
    #[serde(default)]
    pub(crate) low: Vec<WireValue<JsonDecimal>>,
    #[serde(default)]
    pub(crate) close: Vec<WireValue<JsonDecimal>>,
    #[serde(default)]
    pub(crate) volume: Vec<Option<u64>>,
}

#[derive(Deserialize, Clone)]
pub struct AdjCloseBlock {
    #[serde(default)]
    pub(crate) adjclose: Vec<WireValue<JsonDecimal>>,
}

#[derive(Deserialize, Default, Clone)]
pub struct Events {
    #[serde(default)]
    pub(crate) dividends: Option<BTreeMap<String, DividendEvent>>,
    #[serde(default)]
    pub(crate) splits: Option<BTreeMap<String, SplitEvent>>,
    #[serde(default, rename = "capitalGains")]
    pub(crate) capital_gains: Option<BTreeMap<String, CapitalGainEvent>>,
}

#[derive(Deserialize, Clone)]
pub struct DividendEvent {
    #[serde(default)]
    pub(crate) amount: WireValue<JsonDecimal>,
    pub(crate) date: Option<i64>,
    pub(crate) currency: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct SplitEvent {
    #[serde(default, deserialize_with = "de_opt_decimal_from_mixed")]
    pub(crate) numerator: Option<Decimal>,
    #[serde(default, deserialize_with = "de_opt_decimal_from_mixed")]
    pub(crate) denominator: Option<Decimal>,
    #[serde(rename = "splitRatio")]
    pub(crate) split_ratio: Option<String>,
    pub(crate) date: Option<i64>,
}

#[derive(Deserialize, Clone)]
pub struct CapitalGainEvent {
    #[serde(default)]
    pub(crate) amount: WireValue<JsonDecimal>,
    pub(crate) date: Option<i64>,
    pub(crate) currency: Option<String>,
}

/// Accepts numeric split components as integers, decimals, numeric strings, or null/missing.
///
/// Yahoo can emit fractional split components for preferred-share adjustments
/// (for example `1.262838`). Keep the wire layer exact and normalize the
/// pair later, where both numerator and denominator are available together.
fn de_opt_decimal_from_mixed<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    de_opt_decimal(deserializer)
}

fn de_opt_decimal<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    WireValue::<JsonDecimal>::deserialize(deserializer)
        .map(|value| value.into_option().map(JsonDecimal::into_decimal))
}
