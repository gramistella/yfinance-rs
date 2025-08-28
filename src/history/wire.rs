use serde::Deserialize;
use serde::Deserializer;
use std::collections::BTreeMap;

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
    #[serde(default, rename = "capitalGains")]
    pub(crate) capital_gains: Option<BTreeMap<String, CapitalGainEvent>>,
}

#[derive(Deserialize)]
pub(crate) struct DividendEvent {
    pub(crate) amount: Option<f64>,
    pub(crate) date: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct SplitEvent {
    #[serde(default, deserialize_with = "de_opt_u64_from_mixed")]
    pub(crate) numerator: Option<u64>,
    #[serde(default, deserialize_with = "de_opt_u64_from_mixed")]
    pub(crate) denominator: Option<u64>,
    #[serde(rename = "splitRatio")]
    pub(crate) split_ratio: Option<String>,
    pub(crate) date: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct CapitalGainEvent {
    pub(crate) amount: Option<f64>,
    pub(crate) date: Option<i64>,
}

/// Accepts u64, integer-like f64 (e.g., 4.0), numeric strings ("4"), or null/missing.
/// Rounds floats and rejects non-finite or clearly non-integer floats.
fn de_opt_u64_from_mixed<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;

    let v = Option::<Value>::deserialize(deserializer)?;
    let some = match v {
        None => return Ok(None),
        Some(Value::Null) => return Ok(None),
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                Some(u)
            } else if let Some(f) = n.as_f64() {
                if !f.is_finite() {
                    return Err(D::Error::custom("non-finite float for split field"));
                }
                let r = f.round();
                // Require the float to be very close to an integer
                if (f - r).abs() < 1e-9 && r >= 0.0 {
                    Some(r as u64)
                } else {
                    return Err(D::Error::custom(format!(
                        "expected integer-like float for split field, got {f}"
                    )));
                }
            } else {
                return Err(D::Error::custom("unsupported number type for split field"));
            }
        }
        Some(Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                match s.parse::<u64>() {
                    Ok(u) => Some(u),
                    Err(_) => {
                        return Err(D::Error::custom(format!(
                            "invalid numeric string '{s}' for split field"
                        )));
                    }
                }
            }
        }
        Some(other) => {
            return Err(D::Error::custom(format!(
                "unexpected JSON type for split field: {other}"
            )));
        }
    };
    Ok(some)
}
