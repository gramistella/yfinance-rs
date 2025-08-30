use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Clone, Copy)]
pub struct RawNum<T> {
    pub(crate) raw: Option<T>,
}

pub fn from_raw<T>(raw: Option<RawNum<T>>) -> Option<T> {
    raw.and_then(|n| n.raw)
}

pub fn from_raw_u32_round(r: Option<RawNum<f64>>) -> Option<u32> {
    r.and_then(|n| n.raw).and_then(|v| {
        let rounded = v.round();
        if rounded >= 0.0 && rounded <= f64::from(u32::MAX) {
            // This cast is safe as we check the bounds of rounded.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Some(rounded as u32)
        } else {
            None
        }
    })
}

#[derive(Deserialize, Clone, Copy)]
pub struct RawDate {
    pub(crate) raw: Option<i64>,
}

pub fn from_raw_date(r: Option<RawDate>) -> Option<i64> {
    r.and_then(|d| d.raw)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn de_u64_from_any_number<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AnyNumber {
        U64(u64),
        F64(f64),
    }

    match Option::<AnyNumber>::deserialize(deserializer)? {
        Some(AnyNumber::U64(u)) => Ok(Some(u)),
        Some(AnyNumber::F64(f)) => {
            if f.fract() == 0.0 && f >= 0.0 {
                Ok(Some(f as u64))
            } else {
                Err(serde::de::Error::custom(format!(
                    "cannot convert float {f} to u64"
                )))
            }
        }
        None => Ok(None),
    }
}

#[derive(Deserialize, Clone, Copy)]
pub struct RawNumU64 {
    #[serde(deserialize_with = "de_u64_from_any_number")]
    pub(crate) raw: Option<u64>,
}
