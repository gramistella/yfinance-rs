//! Conversion utilities between old valuta types and new paft types

use chrono::{DateTime, Utc};
use paft::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;

fn f64_to_decimal_safely(value: f64) -> rust_decimal::Decimal {
    if !value.is_finite() {
        return rust_decimal::Decimal::ZERO;
    }
    let formatted = format!("{value:.4}");
    rust_decimal::Decimal::from_str(&formatted).unwrap_or(rust_decimal::Decimal::ZERO)
}

/// Convert f64 to Money with specified currency
#[must_use] pub fn f64_to_money_with_currency(value: f64, currency: Currency) -> Money {
    // Use string formatting to avoid f64 precision issues; coerce non-finite to zero
    let decimal = f64_to_decimal_safely(value);
    Money::new(decimal, currency)
}

/// Convert i64 to Money with specified currency (no precision loss)
#[must_use]
pub fn i64_to_money_with_currency(value: i64, currency: Currency) -> Money {
    let decimal = rust_decimal::Decimal::from_i128_with_scale(i128::from(value), 0);
    Money::new(decimal, currency)
}

/// Convert u64 to Money with specified currency (no precision loss)
#[must_use]
pub fn u64_to_money_with_currency(value: u64, currency: Currency) -> Money {
    let decimal = rust_decimal::Decimal::from_i128_with_scale(i128::from(value), 0);
    Money::new(decimal, currency)
}

/// Convert f64 to Money with currency string (parses currency string to Currency enum)
#[must_use] pub fn f64_to_money_with_currency_str(value: f64, currency_str: Option<&str>) -> Money {
    let currency = currency_str
        .and_then(|s| Currency::from_str(s).ok())
        .unwrap_or(Currency::USD);
    f64_to_money_with_currency(value, currency)
}

/// Convert Money to f64 (loses currency information)
#[must_use] pub fn money_to_f64(money: &Money) -> f64 {
    money.amount().to_f64().unwrap_or(0.0)
}

/// Extract currency string from Money object
#[must_use] pub fn money_to_currency_str(money: &Money) -> Option<String> {
    Some(money.currency().to_string())
}

/// Convert i64 timestamp to `DateTime`<Utc>
#[must_use] pub fn i64_to_datetime(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).unwrap_or_default()
}

/// Convert `DateTime`<Utc> to i64 timestamp
#[must_use] pub const fn datetime_to_i64(dt: DateTime<Utc>) -> i64 {
    dt.timestamp()
}

/// Convert String to Exchange enum
#[allow(clippy::single_option_map)]
#[must_use] pub fn string_to_exchange(s: Option<String>) -> Option<Exchange> {
    s.map(|s| {
        // Map Yahoo Finance exchange names to paft Exchange values
        match s.as_str() {
            "NasdaqGS" | "NasdaqCM" | "NasdaqGM" => Exchange::NASDAQ,
            "NYSE" => Exchange::NYSE,
            "AMEX" => Exchange::AMEX,
            "BATS" => Exchange::BATS,
            "OTC" => Exchange::OTC,
            "LSE" => Exchange::LSE,
            "TSE" => Exchange::TSE,
            "HKEX" => Exchange::HKEX,
            "SSE" => Exchange::SSE,
            "SZSE" => Exchange::SZSE,
            "TSX" => Exchange::TSX,
            "ASX" => Exchange::ASX,
            "Euronext" => Exchange::Euronext,
            "XETRA" => Exchange::XETRA,
            "SIX" => Exchange::SIX,
            "BIT" => Exchange::BIT,
            "BME" => Exchange::BME,
            "AEX" => Exchange::AEX,
            "BRU" => Exchange::BRU,
            "LIS" => Exchange::LIS,
            "EPA" => Exchange::EPA,
            "OSL" => Exchange::OSL,
            "STO" => Exchange::STO,
            "CPH" => Exchange::CPH,
            "WSE" => Exchange::WSE,
            "PSE" => Exchange::PSE,
            "BSE" => Exchange::BSE,
            "MOEX" => Exchange::MOEX,
            "BIST" => Exchange::BIST,
            "JSE" => Exchange::JSE,
            "TASE" => Exchange::TASE,
            "BseIndia" => Exchange::BseIndia,
            "NSE" => Exchange::NSE,
            "KRX" => Exchange::KRX,
            "SGX" => Exchange::SGX,
            "SET" => Exchange::SET,
            "KLSE" => Exchange::KLSE,
            "PsePhil" => Exchange::PsePhil,
            "IDX" => Exchange::IDX,
            "HOSE" => Exchange::HOSE,
            _ => Exchange::from(s), // Fallback to paft's parsing
        }
    })
}

/// Convert Exchange to String
#[must_use] pub fn exchange_to_string(exchange: Option<Exchange>) -> Option<String> {
    exchange.map(|e| e.to_string())
}

/// Convert String to `MarketState` enum
pub fn string_to_market_state(s: Option<String>) -> Option<MarketState> {
    s.map(MarketState::from)
}

/// Convert `MarketState` to String
#[must_use] pub fn market_state_to_string(state: Option<MarketState>) -> Option<String> {
    state.map(|s| s.to_string())
}

/// Convert String to `FundKind` enum
#[allow(clippy::single_option_map)]
#[must_use] pub fn string_to_fund_kind(s: Option<String>) -> Option<FundKind> {
    s.map(|s| {
        // Map Yahoo Finance legal types to paft FundKind values
        match s.as_str() {
            "Exchange Traded Fund" => FundKind::Etf,
            "Mutual Fund" => FundKind::MutualFund,
            "Index Fund" => FundKind::IndexFund,
            "Closed-End Fund" => FundKind::ClosedEndFund,
            "Money Market Fund" => FundKind::MoneyMarketFund,
            "Hedge Fund" => FundKind::HedgeFund,
            "Real Estate Investment Trust" => FundKind::Reit,
            "Unit Investment Trust" => FundKind::UnitInvestmentTrust,
            _ => FundKind::from(s), // Fallback to paft's parsing
        }
    })
}

/// Convert `FundKind` to String
#[must_use] pub fn fund_kind_to_string(kind: Option<FundKind>) -> Option<String> {
    kind.map(|k| k.to_string())
}

/// Convert String to `InsiderPosition` enum
#[must_use] pub fn string_to_insider_position(s: String) -> InsiderPosition {
    InsiderPosition::from(s)
}

/// Convert String to `TransactionType` enum
#[must_use] pub fn string_to_transaction_type(s: String) -> TransactionType {
    TransactionType::from(s)
}

/// Convert String to Period
#[must_use] pub fn string_to_period(s: String) -> Period {
    Period::try_from(s.clone()).unwrap_or(Period::Other(s))
}

/// Convert String to `RecommendationGrade` enum
#[must_use] pub fn string_to_recommendation_grade(s: String) -> RecommendationGrade {
    RecommendationGrade::from(s)
}

/// Convert String to `RecommendationAction` enum
#[must_use] pub fn string_to_recommendation_action(s: String) -> RecommendationAction {
    RecommendationAction::from(s)
}
