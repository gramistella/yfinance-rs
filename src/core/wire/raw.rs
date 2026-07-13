use std::{fmt, marker::PhantomData};

use paft::Decimal;
use serde::{
    Deserialize, Deserializer,
    de::{IgnoredAny, MapAccess, Visitor},
};
use serde_field_result::{Field, FieldDecode};
use serde_json::value::RawValue;

use super::{
    number::{JsonDecimal, JsonU64, de_decimal_from_json, de_u64_from_json},
    value::WireValue,
};

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct RawNum<T> {
    pub(crate) raw: Option<T>,
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct RawDecimal {
    #[serde(default, deserialize_with = "de_decimal_from_json")]
    pub(crate) raw: Option<Decimal>,
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct RawDate {
    pub(crate) raw: Option<i64>,
}

pub fn from_raw_date(r: Option<RawDate>) -> Option<i64> {
    r.and_then(|d| d.raw)
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct RawNumU64 {
    #[serde(default, deserialize_with = "de_u64_from_json")]
    pub(crate) raw: Option<u64>,
}

impl<'de, T> FieldDecode<'de> for RawNum<T>
where
    T: for<'wire> FieldDecode<'wire>,
{
    fn decode_field<D>(deserializer: D) -> Result<Field<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        decode_raw_map::<_, T>(deserializer).map(|decoded| decoded.map(|raw| Self { raw }))
    }
}

impl<'de> FieldDecode<'de> for RawDecimal {
    fn decode_field<D>(deserializer: D) -> Result<Field<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        decode_raw_map::<_, JsonDecimal>(deserializer).map(|decoded| {
            decoded.map(|raw| Self {
                raw: raw.map(JsonDecimal::into_decimal),
            })
        })
    }
}

impl<'de> FieldDecode<'de> for RawDate {
    fn decode_field<D>(deserializer: D) -> Result<Field<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        decode_raw_map::<_, i64>(deserializer).map(|decoded| decoded.map(|raw| Self { raw }))
    }
}

impl<'de> FieldDecode<'de> for RawNumU64 {
    fn decode_field<D>(deserializer: D) -> Result<Field<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        decode_raw_map::<_, JsonU64>(deserializer).map(|decoded| {
            decoded.map(|raw| Self {
                raw: raw.map(JsonU64::into_u64),
            })
        })
    }
}

fn decode_raw_map<'de, D, T>(deserializer: D) -> Result<Field<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: for<'wire> FieldDecode<'wire>,
{
    let raw = Box::<RawValue>::deserialize(deserializer)?;
    Ok(match raw.get().as_bytes().first() {
        None => invalid_raw("empty JSON value"),
        Some(b'n') if raw.get() == "null" => Field::Missing,
        Some(b'{') => decode_raw_object(raw.get()),
        Some(b'[') => invalid_raw("array"),
        Some(b'\"') => invalid_raw("string"),
        Some(b't' | b'f') => invalid_raw("boolean"),
        Some(_) => invalid_raw(format_args!("number `{}`", raw.get())),
    })
}

fn decode_raw_object<T>(value: &str) -> Field<Option<T>>
where
    T: for<'wire> FieldDecode<'wire>,
{
    struct RawMapVisitor<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for RawMapVisitor<T>
    where
        T: FieldDecode<'de>,
    {
        type Value = Field<Option<T>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(RAW_OBJECT)
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut raw = None;
            let mut invalid = None;

            while let Some(field) = map.next_key::<RawField>()? {
                match field {
                    RawField::Raw => match map.next_value::<WireValue<T>>()? {
                        WireValue::Missing => raw = None,
                        WireValue::Valid(value) => raw = Some(value),
                        WireValue::Invalid(error) => {
                            if invalid.is_none() {
                                invalid = Some(error);
                            }
                        }
                    },
                    RawField::Other => {
                        let _: IgnoredAny = map.next_value()?;
                    }
                }
            }

            Ok(invalid.map_or_else(|| Field::Valid(raw), Field::Invalid))
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(value);
    match deserializer.deserialize_map(RawMapVisitor::<T>(PhantomData)) {
        Ok(decoded) => decoded,
        Err(error) => Field::invalid(error.to_string()),
    }
}

const RAW_OBJECT: &str = "Yahoo raw value object";

fn invalid_raw<T>(actual: impl fmt::Display) -> Field<T> {
    Field::invalid(unexpected(RAW_OBJECT, actual))
}

fn unexpected(expected: &'static str, actual: impl fmt::Display) -> String {
    format!("expected {expected}, got {actual}")
}

#[derive(Deserialize)]
#[serde(field_identifier)]
enum RawField {
    #[serde(rename = "raw")]
    Raw,
    #[serde(other)]
    Other,
}
