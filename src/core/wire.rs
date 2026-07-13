mod number;
mod raw;
mod value;

pub use number::parse_decimal_lexeme;
pub use number::{JsonDecimal, JsonU64};
pub use raw::{RawDate, RawDecimal, RawNum, RawNumU64, from_raw_date};
pub use value::{BorrowedWireValue, BufferedWireValue, WireField, WireValue};

#[cfg(test)]
mod tests {
    use super::*;
    use paft::Decimal;
    use serde::Deserialize;
    use std::str::FromStr;

    #[test]
    fn invalid_scalar_is_recorded_without_losing_following_fields() {
        #[derive(Deserialize)]
        struct Row {
            #[serde(default)]
            bad: WireValue<i64>,
            #[serde(default)]
            after: WireValue<i64>,
        }

        let row: Row = serde_json::from_str(r#"{"bad":{"nested":[1,2,3]},"after":7}"#).unwrap();

        assert!(matches!(row.bad, WireValue::Invalid(_)));
        assert!(matches!(row.after, WireValue::Valid(7)));
    }

    #[test]
    fn invalid_raw_value_is_recorded_without_losing_following_fields() {
        #[derive(Deserialize)]
        struct Row {
            #[serde(default)]
            quote: WireValue<RawNum<i64>>,
            #[serde(default)]
            after: WireValue<String>,
        }

        let row: Row =
            serde_json::from_str(r#"{"quote":{"raw":[1,2],"fmt":"bad"},"after":"ok"}"#).unwrap();

        assert!(matches!(row.quote, WireValue::Invalid(_)));
        assert_eq!(row.after.as_str(), Some("ok"));
    }

    #[test]
    fn arbitrary_precision_number_is_invalid_when_raw_object_is_required() {
        #[derive(Deserialize)]
        struct Row {
            #[serde(default)]
            quote: WireValue<RawDecimal>,
            #[serde(default)]
            after: WireValue<String>,
        }

        for literal in ["1.5", "1e20"] {
            let row: Row =
                serde_json::from_str(&format!(r#"{{"quote":{literal},"after":"ok"}}"#)).unwrap();

            assert!(matches!(row.quote, WireValue::Invalid(_)), "{literal}");
            assert_eq!(row.after.as_str(), Some("ok"));
        }
    }

    #[test]
    fn json_u64_accepts_integral_strings_and_rejects_fractional_strings() {
        let valid: WireValue<JsonU64> = serde_json::from_str(r#""42""#).unwrap();
        assert_eq!(valid.as_ref().copied().map(JsonU64::into_u64), Some(42));

        let invalid: WireValue<JsonU64> = serde_json::from_str(r#""42.5""#).unwrap();
        assert!(matches!(
            invalid.invalid_details(),
            Some(details) if details.contains("cannot convert decimal")
        ));

        for literal in ["-0", "-0.0", "-0e999"] {
            let zero: WireValue<JsonU64> = serde_json::from_str(literal).unwrap();
            assert_eq!(
                zero.as_ref().copied().map(JsonU64::into_u64),
                Some(0),
                "{literal}"
            );
        }

        let negative: WireValue<JsonU64> = serde_json::from_str("-1").unwrap();
        assert!(matches!(negative, WireValue::Invalid(_)));

        let buffered: serde_json::Value = serde_json::from_str("-0").unwrap();
        let zero: WireValue<JsonU64> = serde_json::from_value(buffered).unwrap();
        assert_eq!(zero.as_ref().copied().map(JsonU64::into_u64), Some(0));
    }

    #[test]
    fn json_decimal_preserves_the_numeric_lexeme() {
        let value: WireValue<JsonDecimal> =
            serde_json::from_str("0.1234567890123456789012345678").unwrap();

        assert_eq!(
            value.as_ref().copied().map(JsonDecimal::into_decimal),
            Some(Decimal::from_str("0.1234567890123456789012345678").unwrap())
        );
    }

    #[test]
    fn json_decimal_preserves_numeric_strings() {
        let value: WireValue<JsonDecimal> =
            serde_json::from_str(r#"" 0.11049100011587143 ""#).unwrap();

        assert_eq!(
            value.as_ref().copied().map(JsonDecimal::into_decimal),
            Some(Decimal::from_str("0.11049100011587143").unwrap())
        );
    }

    #[test]
    fn json_decimal_preserves_representable_wire_scale() {
        for (literal, expected) in [
            ("189.50", "189.50"),
            ("0.00", "0.00"),
            ("1.2300e2", "123.00"),
        ] {
            let value: WireValue<JsonDecimal> = serde_json::from_str(literal).unwrap();
            assert_eq!(
                value
                    .as_ref()
                    .copied()
                    .map(JsonDecimal::into_decimal)
                    .map(|value| value.to_string()),
                Some(expected.to_string()),
                "{literal}"
            );
        }
    }

    #[test]
    fn json_decimal_accepts_all_exactly_representable_exponent_forms() {
        for (literal, expected) in [
            ("1.0e-28", "0.0000000000000000000000000001"),
            ("0.1e29", "10000000000000000000000000000"),
            (
                "792281625142643375935439503350e-1",
                "79228162514264337593543950335",
            ),
            (
                "0.1234567890123456789012345678000",
                "0.1234567890123456789012345678",
            ),
        ] {
            let value: WireValue<JsonDecimal> = serde_json::from_str(literal).unwrap();
            assert_eq!(
                value.as_ref().copied().map(JsonDecimal::into_decimal),
                Some(Decimal::from_str(expected).unwrap()),
                "{literal}"
            );
        }
    }

    #[test]
    fn json_decimal_rejects_values_that_would_need_rounding() {
        for literal in [
            "1e-29",
            "1.23456789012345678901234567895",
            "79228162514264337593543950336",
        ] {
            let value: WireValue<JsonDecimal> = serde_json::from_str(literal).unwrap();
            assert!(
                matches!(value, WireValue::Invalid(_)),
                "{literal} must not be rounded"
            );
        }
    }

    #[test]
    fn json_decimal_accepts_zero_with_an_unbounded_exponent() {
        let value: WireValue<JsonDecimal> =
            serde_json::from_str("0e999999999999999999999999999999999999999999999999999999999999")
                .unwrap();

        assert_eq!(
            value.as_ref().copied().map(JsonDecimal::into_decimal),
            Some(Decimal::ZERO)
        );
    }

    #[test]
    fn json_decimal_remains_exact_after_a_value_round_trip() {
        for literal in ["0.11049100011587143", "0.1234567890123456789012345678"] {
            let buffered: serde_json::Value = serde_json::from_str(literal).unwrap();
            let value: WireValue<JsonDecimal> = serde_json::from_value(buffered).unwrap();

            assert_eq!(
                value.as_ref().copied().map(JsonDecimal::into_decimal),
                Some(Decimal::from_str(literal).unwrap())
            );
        }
    }

    #[test]
    fn raw_decimal_preserves_the_numeric_lexeme() {
        #[derive(Deserialize)]
        struct Row {
            #[serde(default)]
            quote: WireValue<RawDecimal>,
        }

        let row: Row = serde_json::from_str(
            r#"{"quote":{"raw":0.1234567890123456789012345678,"fmt":"0.12"}}"#,
        )
        .unwrap();

        assert_eq!(
            row.quote.as_ref().and_then(|value| value.raw),
            Some(Decimal::from_str("0.1234567890123456789012345678").unwrap())
        );
    }

    #[test]
    fn json_u64_preserves_the_full_unsigned_range() {
        let integer: WireValue<JsonU64> = serde_json::from_str("18446744073709551615").unwrap();
        let decimal: WireValue<JsonU64> = serde_json::from_str("18446744073709551615.0").unwrap();

        assert_eq!(
            integer.as_ref().copied().map(JsonU64::into_u64),
            Some(u64::MAX)
        );
        assert_eq!(
            decimal.as_ref().copied().map(JsonU64::into_u64),
            Some(u64::MAX)
        );
    }

    #[test]
    fn invalid_exact_number_is_recorded_without_losing_following_fields() {
        #[derive(Deserialize)]
        struct Row {
            #[serde(default)]
            number: WireValue<JsonDecimal>,
            #[serde(default)]
            after: WireValue<i64>,
        }

        let row: Row =
            serde_json::from_str(r#"{"number":{"$serde_json::private::Number":"0.5"},"after":11}"#)
                .unwrap();

        assert!(matches!(row.number, WireValue::Invalid(_)));
        assert!(matches!(row.after, WireValue::Valid(11)));
    }

    #[test]
    fn buffered_wire_value_keeps_composite_recovery_explicit() {
        #[allow(dead_code)]
        #[derive(Deserialize)]
        struct Nested {
            value: String,
        }

        #[derive(Deserialize)]
        struct Row {
            #[serde(default)]
            nested: BufferedWireValue<Nested>,
            #[serde(default)]
            after: WireValue<i64>,
        }

        let row: Row = serde_json::from_str(r#"{"nested":{"value":[]},"after":9}"#).unwrap();

        assert!(matches!(row.nested, BufferedWireValue::Invalid { .. }));
        assert!(matches!(row.after, WireValue::Valid(9)));
    }
}
