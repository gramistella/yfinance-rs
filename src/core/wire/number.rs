use paft::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Deserializer, de};
use serde_field_result::{Field, FieldDecode, FieldError};
use serde_json::value::RawValue;

#[derive(Clone, Copy, Debug)]
pub struct JsonDecimal {
    value: Decimal,
}

impl JsonDecimal {
    pub(crate) const fn into_decimal(self) -> Decimal {
        self.value
    }
}

impl<'de> Deserialize<'de> for JsonDecimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match <Self as FieldDecode<'de>>::decode_field(deserializer)? {
            Field::Valid(value) => Ok(value),
            Field::Missing => Err(de::Error::invalid_type(
                de::Unexpected::Unit,
                &"JSON number or numeric string",
            )),
            Field::Invalid(error) => Err(de::Error::custom(error)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct JsonU64 {
    value: u64,
}

impl JsonU64 {
    pub(crate) const fn into_u64(self) -> u64 {
        self.value
    }
}

impl<'de> Deserialize<'de> for JsonU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match <Self as FieldDecode<'de>>::decode_field(deserializer)? {
            Field::Valid(value) => Ok(value),
            Field::Missing => Err(de::Error::invalid_type(
                de::Unexpected::Unit,
                &"unsigned integer or unsigned integer string",
            )),
            Field::Invalid(error) => Err(de::Error::custom(error)),
        }
    }
}

pub(super) fn de_decimal_from_json<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<JsonDecimal>::deserialize(deserializer)
        .map(|value| value.map(JsonDecimal::into_decimal))
}

pub(super) fn de_u64_from_json<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<JsonU64>::deserialize(deserializer).map(|value| value.map(JsonU64::into_u64))
}

fn decimal_from_str_field(value: &str) -> Result<Decimal, FieldError> {
    parse_decimal_lexeme(value)
        .map_err(|err| FieldError::new(format!("cannot parse decimal {value:?}: {err}")))
}

/// Parses decimal text without rounding digits that do not fit `Decimal`.
///
/// Yahoo uses both JSON numbers and numeric strings, including exponent form.
/// `rust_decimal`'s ordinary `FromStr` implementation is intentionally lossy at
/// the precision boundary, so provider text must not use it directly.
pub fn parse_decimal_lexeme(value: &str) -> Result<Decimal, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("empty numeric value".into());
    }

    let (coefficient, exponent) = split_exponent(value)?;
    let (negative, coefficient) = strip_sign(coefficient);
    let (mut digits, fractional_digits) = coefficient_digits(coefficient)?;
    let exponent = exponent_parts(exponent)?;

    let first_significant = digits.iter().position(|digit| *digit != b'0');
    let Some(first_significant) = first_significant else {
        return Ok(zero_with_wire_scale(fractional_digits, exponent));
    };
    digits.drain(..first_significant);

    let exponent = signed_exponent(exponent)?;
    let mut power = exponent
        .checked_sub(
            i128::try_from(fractional_digits)
                .map_err(|_| "fractional digit count overflow".to_string())?,
        )
        .ok_or_else(|| "decimal exponent overflow".to_string())?;

    loop {
        match decimal_from_coefficient(&digits, power, negative) {
            Ok(value) => return Ok(value),
            Err(_) if power < 0 && digits.last() == Some(&b'0') => {
                digits.pop();
                power = power
                    .checked_add(1)
                    .ok_or_else(|| "decimal exponent overflow".to_string())?;
            }
            Err(error) => return Err(error),
        }
    }
}

fn zero_with_wire_scale(fractional_digits: usize, exponent: Option<(bool, &str)>) -> Decimal {
    let Ok(exponent) = signed_exponent(exponent) else {
        return Decimal::ZERO;
    };
    let Ok(fractional_digits) = i128::try_from(fractional_digits) else {
        return Decimal::ZERO;
    };
    let Some(power) = exponent.checked_sub(fractional_digits) else {
        return Decimal::ZERO;
    };
    let Some(scale) = power
        .checked_neg()
        .filter(|_| power < 0)
        .and_then(|scale| u32::try_from(scale).ok())
        .filter(|scale| *scale <= Decimal::MAX_SCALE)
    else {
        return Decimal::ZERO;
    };

    Decimal::try_from_i128_with_scale(0, scale).unwrap_or(Decimal::ZERO)
}

fn decimal_from_coefficient(digits: &[u8], power: i128, negative: bool) -> Result<Decimal, String> {
    if digits.len() > 29 {
        return Err("significant coefficient exceeds Decimal's 96-bit mantissa".into());
    }

    let coefficient = std::str::from_utf8(digits)
        .map_err(|_| "numeric coefficient is not ASCII".to_string())?
        .parse::<i128>()
        .map_err(|err| format!("invalid numeric coefficient: {err}"))?;
    let coefficient = if negative {
        coefficient
            .checked_neg()
            .ok_or_else(|| "numeric coefficient overflow".to_string())?
    } else {
        coefficient
    };

    let (mantissa, scale) = if power >= 0 {
        let power = u32::try_from(power)
            .map_err(|_| "decimal magnitude exceeds Decimal's range".to_string())?;
        let multiplier = 10_i128
            .checked_pow(power)
            .ok_or_else(|| "decimal magnitude exceeds Decimal's range".to_string())?;
        let mantissa = coefficient
            .checked_mul(multiplier)
            .ok_or_else(|| "decimal magnitude exceeds Decimal's range".to_string())?;
        (mantissa, 0)
    } else {
        let scale = u32::try_from(
            power
                .checked_neg()
                .ok_or_else(|| "decimal scale overflow".to_string())?,
        )
        .map_err(|_| "decimal scale exceeds Decimal's range".to_string())?;
        (coefficient, scale)
    };

    Decimal::try_from_i128_with_scale(mantissa, scale)
        .map_err(|err| format!("value does not fit Decimal exactly: {err}"))
}

fn split_exponent(value: &str) -> Result<(&str, Option<&str>), String> {
    let mut parts = value.split(['e', 'E']);
    let coefficient = parts.next().expect("split always yields one part");
    let exponent = parts.next();
    if parts.next().is_some() {
        return Err("multiple exponent markers".into());
    }
    Ok((coefficient, exponent))
}

fn strip_sign(value: &str) -> (bool, &str) {
    value.strip_prefix('-').map_or_else(
        || (false, value.strip_prefix('+').unwrap_or(value)),
        |value| (true, value),
    )
}

fn coefficient_digits(value: &str) -> Result<(Vec<u8>, usize), String> {
    let mut digits = Vec::with_capacity(value.len());
    let mut decimal_point = None;

    for byte in value.bytes() {
        match byte {
            b'0'..=b'9' => digits.push(byte),
            b'.' if decimal_point.is_none() => decimal_point = Some(digits.len()),
            b'.' => return Err("multiple decimal points".into()),
            _ => {
                return Err(format!(
                    "invalid character {:?} in coefficient",
                    char::from(byte)
                ));
            }
        }
    }
    if digits.is_empty() {
        return Err("numeric coefficient has no digits".into());
    }

    let fractional_digits = decimal_point.map_or(0, |point| digits.len() - point);
    Ok((digits, fractional_digits))
}

fn exponent_parts(exponent: Option<&str>) -> Result<Option<(bool, &str)>, String> {
    let Some(exponent) = exponent else {
        return Ok(None);
    };
    let (negative, digits) = strip_sign(exponent);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid decimal exponent".into());
    }
    Ok(Some((negative, digits)))
}

fn signed_exponent(exponent: Option<(bool, &str)>) -> Result<i128, String> {
    let Some((negative, digits)) = exponent else {
        return Ok(0);
    };
    let magnitude = digits
        .parse::<i128>()
        .map_err(|_| "decimal exponent exceeds supported range".to_string())?;
    if negative {
        magnitude
            .checked_neg()
            .ok_or_else(|| "decimal exponent exceeds supported range".to_string())
    } else {
        Ok(magnitude)
    }
}

fn u64_from_decimal(decimal: Decimal) -> Result<u64, FieldError> {
    if decimal.is_sign_negative() || !decimal.fract().is_zero() {
        return Err(FieldError::new(format!(
            "cannot convert decimal {decimal} to u64"
        )));
    }
    decimal
        .to_u64()
        .ok_or_else(|| FieldError::new(format!("cannot convert decimal {decimal} to u64")))
}

trait JsonNumeric: Sized {
    const EXPECTED: &'static str = "JSON number or numeric string";

    fn from_str(value: &str) -> Result<Self, FieldError>;
}

impl JsonNumeric for JsonDecimal {
    fn from_str(value: &str) -> Result<Self, FieldError> {
        decimal_from_str_field(value.trim()).map(|value| Self { value })
    }
}

impl JsonNumeric for JsonU64 {
    const EXPECTED: &'static str = "unsigned integer or unsigned integer string";

    fn from_str(value: &str) -> Result<Self, FieldError> {
        let value = value.trim();
        if value.starts_with('-') {
            let decimal = decimal_from_str_field(value)?;
            if decimal.is_zero() {
                return Ok(Self { value: 0 });
            }
            return Err(FieldError::new(format!(
                "cannot convert signed value {value:?} to u64"
            )));
        }
        if let Ok(value) = value.parse() {
            return Ok(Self { value });
        }

        let decimal = decimal_from_str_field(value)?;
        u64_from_decimal(decimal).map(|value| Self { value })
    }
}

impl<'de> FieldDecode<'de> for JsonDecimal {
    fn decode_field<D>(deserializer: D) -> Result<Field<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        decode_json_numeric(deserializer)
    }
}

impl<'de> FieldDecode<'de> for JsonU64 {
    fn decode_field<D>(deserializer: D) -> Result<Field<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        decode_json_numeric(deserializer)
    }
}

fn decode_json_numeric<'de, D, T>(deserializer: D) -> Result<Field<T>, D::Error>
where
    D: Deserializer<'de>,
    T: JsonNumeric,
{
    let raw = Box::<RawValue>::deserialize(deserializer)?;
    Ok(decode_raw_json_numeric(raw.get()))
}

fn decode_raw_json_numeric<T>(raw: &str) -> Field<T>
where
    T: JsonNumeric,
{
    match raw.as_bytes().first() {
        None => Field::invalid(FieldError::static_message("empty JSON value")),
        Some(b'n') if raw == "null" => Field::Missing,
        Some(b'\"') => match serde_json::from_str::<String>(raw) {
            Ok(value) => Field::from_result(T::from_str(&value)),
            Err(error) => Field::invalid(FieldError::new(error.to_string())),
        },
        Some(b'{') => invalid_numeric::<T>("object"),
        Some(b'[') => invalid_numeric::<T>("array"),
        Some(b't' | b'f') => invalid_numeric::<T>("boolean"),
        Some(_) => Field::from_result(T::from_str(raw)),
    }
}

fn invalid_numeric<T: JsonNumeric>(actual: &str) -> Field<T> {
    Field::invalid(FieldError::new(format!(
        "expected {}, got {actual}",
        T::EXPECTED
    )))
}
