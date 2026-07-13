use paft::Decimal;
use paft::domain::{AssetKind, Exchange, ReportingPeriod};
use paft::fundamentals::{
    analysis::{RecommendationAction, RecommendationGrade},
    holders::{InsiderPosition, TransactionType},
    profile::FundKind,
};
use paft::money::{Currency, IsoCurrency};
use std::str::FromStr;
use yfinance_rs::YfError;
use yfinance_rs::core::conversions::{
    decimal_from_f32, i64_to_datetime, i64_to_money_with_currency,
    money_from_decimal_with_currency_str, price_from_decimal_with_currency_str,
    string_to_asset_kind, string_to_fund_kind, string_to_insider_position, string_to_period,
    string_to_recommendation_action, string_to_recommendation_grade, string_to_transaction_type,
    u64_to_money_with_currency,
};
use yfinance_rs::core::yahoo_vocab::{parse_yahoo_exchange, yahoo_exchange_to_listing_currency};

#[test]
fn invalid_protobuf_float_conversions_return_none() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(decimal_from_f32(value).is_none());
    }
}

#[test]
fn protobuf_f32_decimal_conversion_uses_source_precision() {
    assert_eq!(decimal_from_f32(314.6_f32).unwrap().to_string(), "314.6");
}

#[test]
fn valid_zero_remains_zero() {
    let price = price_from_decimal_with_currency_str(Decimal::ZERO, Some("USD"))
        .expect("zero is a valid price");
    assert_eq!(price.amount(), paft::Decimal::from(0));
}

#[test]
fn missing_or_invalid_currency_string_conversions_return_none() {
    assert!(money_from_decimal_with_currency_str(Decimal::ONE, None).is_none());
    assert!(price_from_decimal_with_currency_str(Decimal::ONE, None).is_none());
    assert!(money_from_decimal_with_currency_str(Decimal::ONE, Some("")).is_none());
    assert!(price_from_decimal_with_currency_str(Decimal::ONE, Some("!!!")).is_none());
}

#[test]
fn yahoo_unit_currencies_are_scaled_to_major_units() {
    let gbp = price_from_decimal_with_currency_str(Decimal::from(123), Some("GBp")).unwrap();
    assert_eq!(gbp.currency(), &Currency::Iso(IsoCurrency::GBP));
    assert_eq!(gbp.amount(), paft::Decimal::new(123, 2));

    let zar = price_from_decimal_with_currency_str(Decimal::from(456), Some("ZAc")).unwrap();
    assert_eq!(zar.currency().to_string(), "ZAR");
    assert_eq!(zar.amount(), paft::Decimal::new(456, 2));

    let ils = price_from_decimal_with_currency_str(Decimal::from(789), Some("ILA")).unwrap();
    assert_eq!(ils.currency().to_string(), "ILS");
    assert_eq!(ils.amount(), paft::Decimal::new(789, 2));
}

#[test]
fn yahoo_unit_currencies_do_not_scale_major_money_amounts() {
    let gbp = money_from_decimal_with_currency_str(Decimal::from(123), Some("GBp")).unwrap();
    assert_eq!(gbp.currency(), &Currency::Iso(IsoCurrency::GBP));
    assert_eq!(gbp.amount(), paft::Decimal::from(123));
}

#[test]
fn yahoo_exchange_codes_normalize_to_provider_agnostic_exchanges() {
    for code in ["NMS", "NGM", "NCM", "NAS", "NasdaqGS"] {
        assert_eq!(parse_yahoo_exchange(code).unwrap(), Exchange::NASDAQ);
    }

    assert_eq!(parse_yahoo_exchange("NYQ").unwrap(), Exchange::NYSE);
    assert_eq!(parse_yahoo_exchange("ASE").unwrap(), Exchange::AMEX);
    assert_eq!(parse_yahoo_exchange("BTS").unwrap(), Exchange::BATS);
    assert_eq!(parse_yahoo_exchange("PNK").unwrap(), Exchange::OTC);
    assert_eq!(
        parse_yahoo_exchange("London Stock Exchange").unwrap(),
        Exchange::LSE
    );
    assert_eq!(parse_yahoo_exchange("JPX").unwrap(), Exchange::TSE);
    assert_eq!(parse_yahoo_exchange("GER").unwrap(), Exchange::XETRA);
    assert_eq!(
        parse_yahoo_exchange("PCX").unwrap().to_string(),
        "NYSE_ARCA"
    );
    assert_eq!(yahoo_exchange_to_listing_currency("PCX"), Some("USD"));
    assert_eq!(
        yahoo_exchange_to_listing_currency("London Stock Exchange"),
        Some("GBp")
    );
    assert!(matches!(
        parse_yahoo_exchange("us_market"),
        Err(YfError::InvalidData(_))
    ));
}

#[test]
fn yahoo_exchange_listing_currency_uses_unparsed_aliases() {
    assert_eq!(yahoo_exchange_to_listing_currency("NASDAQ"), Some("USD"));
    assert_eq!(yahoo_exchange_to_listing_currency("LONDON"), Some("GBp"));
    assert_eq!(yahoo_exchange_to_listing_currency("FRA"), Some("EUR"));
}

#[test]
fn integer_money_conversions_return_errors_for_missing_currency_metadata() {
    let currency = Currency::from_str("ZZZ").unwrap();

    assert!(matches!(
        i64_to_money_with_currency(1, currency.clone()),
        Err(YfError::Money(_))
    ));
    assert!(matches!(
        u64_to_money_with_currency(1, currency),
        Err(YfError::Money(_))
    ));
}

#[test]
fn invalid_timestamps_return_errors() {
    assert!(i64_to_datetime(0).is_ok());
    assert!(matches!(
        i64_to_datetime(i64::MAX),
        Err(YfError::InvalidData(_))
    ));
}

#[test]
fn missing_or_invalid_required_tokens_do_not_default_to_plausible_values() {
    assert!(matches!(string_to_period(""), Err(YfError::MissingData(_))));
    assert!(matches!(
        string_to_recommendation_grade("!!!"),
        Err(YfError::InvalidData(_))
    ));
    assert!(matches!(
        string_to_recommendation_action("!!!"),
        Err(YfError::InvalidData(_))
    ));
    assert!(matches!(
        string_to_insider_position(""),
        Err(YfError::MissingData(_))
    ));
    assert!(matches!(
        string_to_transaction_type(""),
        Err(YfError::MissingData(_))
    ));
    assert!(matches!(
        string_to_asset_kind(""),
        Err(YfError::MissingData(_))
    ));
    assert!(string_to_fund_kind(None).unwrap().is_none());
}

#[test]
fn unknown_nonempty_tokens_are_preserved_as_extensible_other_values() {
    assert!(matches!(
        string_to_period("current quarter").unwrap(),
        ReportingPeriod::Other(_)
    ));
    assert!(matches!(
        string_to_recommendation_grade("conviction buy").unwrap(),
        RecommendationGrade::Other(_)
    ));
    assert!(matches!(
        string_to_recommendation_action("price target raised only").unwrap(),
        RecommendationAction::Other(_)
    ));
    assert!(matches!(
        string_to_insider_position("chief product officer").unwrap(),
        InsiderPosition::Other(_)
    ));
    assert!(matches!(
        string_to_transaction_type("tax withholding").unwrap(),
        TransactionType::Other(_)
    ));
    assert!(matches!(
        string_to_asset_kind("structured note").unwrap(),
        AssetKind::Other(_)
    ));
    assert!(matches!(
        string_to_fund_kind(Some("Interval Fund".to_string()))
            .unwrap()
            .unwrap(),
        FundKind::Other(_)
    ));
}
