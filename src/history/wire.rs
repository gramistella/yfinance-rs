use serde::Deserialize;
use std::collections::BTreeMap;

/* Internal response mapping (only fields we need) */

#[derive(Deserialize)]
pub(crate) struct ChartEnvelope {
    pub(crate) chart: Option<ChartNode>,
}

#[derive(Deserialize)]
pub(crate) struct ChartNode {
    pub(crate) result: Option<Vec<ChartResult>>,
    pub(crate) error: Option<ChartError>,
}

#[derive(Deserialize)]
pub(crate) struct ChartError {
    pub(crate) code: String,
    pub(crate) description: String,
}

#[derive(Deserialize)]
pub(crate) struct ChartResult {
    #[serde(default)]
    pub(crate) meta: Option<MetaNode>,
    #[serde(default)]
    pub(crate) timestamp: Option<Vec<i64>>,
    pub(crate) indicators: Indicators,
    #[serde(default)]
    pub(crate) events: Option<Events>,
}

#[derive(Deserialize)]
pub(crate) struct MetaNode {
    #[serde(default)]
    pub(crate) timezone: Option<String>,
    #[serde(default)]
    pub(crate) gmtoffset: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct Indicators {
    #[serde(default)]
    pub(crate) quote: Vec<QuoteBlock>,
    #[serde(default)]
    pub(crate) adjclose: Vec<AdjCloseBlock>,
}

#[derive(Deserialize)]
pub(crate) struct QuoteBlock {
    #[serde(default)]
    pub(crate) open: Vec<Option<f64>>,
    #[serde(default)]
    pub(crate) high: Vec<Option<f64>>,
    #[serde(default)]
    pub(crate) low: Vec<Option<f64>>,
    #[serde(default)]
    pub(crate) close: Vec<Option<f64>>,
    #[serde(default)]
    pub(crate) volume: Vec<Option<u64>>,
}

#[derive(Deserialize)]
pub(crate) struct AdjCloseBlock {
    #[serde(default)]
    pub(crate) adjclose: Vec<Option<f64>>,
}

#[derive(Deserialize, Default)]
pub(crate) struct Events {
    #[serde(default)]
    pub(crate) dividends: Option<BTreeMap<String, DividendEvent>>,
    #[serde(default)]
    pub(crate) splits: Option<BTreeMap<String, SplitEvent>>,
}

#[derive(Deserialize)]
pub(crate) struct DividendEvent {
    pub(crate) amount: Option<f64>,
    pub(crate) date: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct SplitEvent {
    pub(crate) numerator: Option<u64>,
    pub(crate) denominator: Option<u64>,
    #[serde(rename = "splitRatio")]
    pub(crate) split_ratio: Option<String>,
    pub(crate) date: Option<i64>,
}
