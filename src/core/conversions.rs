//! Conversion utilities

use chrono::{DateTime, Utc};
use paft::domain::{Exchange, MarketState, Period};
use paft::fundamentals::analysis::{RecommendationAction, RecommendationGrade};
use paft::fundamentals::holders::{InsiderPosition, TransactionType};
use paft::fundamentals::profile::FundKind;
use paft::money::{Currency, IsoCurrency, Money};
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
///
/// # Panics
/// Panics if currency metadata is not registered for non-ISO currencies.
#[must_use]
pub fn f64_to_money_with_currency(value: f64, currency: Currency) -> Money {
    // Use string formatting to avoid f64 precision issues; coerce non-finite to zero
    let decimal = f64_to_decimal_safely(value);
    Money::new(decimal, currency).expect("currency metadata available")
}

/// Convert i64 to Money with specified currency (no precision loss)
///
/// # Panics
/// Panics if currency metadata is not registered for non-ISO currencies.
#[must_use]
pub fn i64_to_money_with_currency(value: i64, currency: Currency) -> Money {
    let decimal = rust_decimal::Decimal::from_i128_with_scale(i128::from(value), 0);
    Money::new(decimal, currency).expect("currency metadata available")
}

/// Convert u64 to Money with specified currency (no precision loss)
///
/// # Panics
/// Panics if currency metadata is not registered for non-ISO currencies.
#[must_use]
pub fn u64_to_money_with_currency(value: u64, currency: Currency) -> Money {
    let decimal = rust_decimal::Decimal::from_i128_with_scale(i128::from(value), 0);
    Money::new(decimal, currency).expect("currency metadata available")
}

/// Convert f64 to Money with currency string (parses currency string to Currency enum)
#[must_use]
pub fn f64_to_money_with_currency_str(value: f64, currency_str: Option<&str>) -> Money {
    let currency = currency_str
        .and_then(|s| Currency::from_str(s).ok())
        .unwrap_or(Currency::Iso(IsoCurrency::USD));
    f64_to_money_with_currency(value, currency)
}

/// Convert Money to f64 (loses currency information)
#[must_use]
pub fn money_to_f64(money: &Money) -> f64 {
    money.amount().to_f64().unwrap_or(0.0)
}

/// Extract currency string from Money object
#[must_use]
pub fn money_to_currency_str(money: &Money) -> Option<String> {
    Some(money.currency().to_string())
}

/// Convert i64 timestamp to `DateTime`<Utc>
#[must_use]
pub fn i64_to_datetime(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).unwrap_or_default()
}

/// Convert `DateTime`<Utc> to i64 timestamp
#[must_use]
pub const fn datetime_to_i64(dt: DateTime<Utc>) -> i64 {
    dt.timestamp()
}

/// Convert String to Exchange enum
#[allow(clippy::single_option_map)]
#[must_use]
pub fn string_to_exchange(s: Option<String>) -> Option<Exchange> {
    s.and_then(|s| {
        // Map Yahoo Finance exchange names to paft Exchange values
        match s.as_str() {
            "NasdaqGS" | "NasdaqCM" | "NasdaqGM" => Some(Exchange::NASDAQ),
            "NYSE" => Some(Exchange::NYSE),
            "AMEX" => Some(Exchange::AMEX),
            "BATS" => Some(Exchange::BATS),
            "OTC" => Some(Exchange::OTC),
            "LSE" => Some(Exchange::LSE),
            "TSE" => Some(Exchange::TSE),
            "HKEX" => Some(Exchange::HKEX),
            "SSE" => Some(Exchange::SSE),
            "SZSE" => Some(Exchange::SZSE),
            "TSX" => Some(Exchange::TSX),
            "ASX" => Some(Exchange::ASX),
            "Euronext" => Some(Exchange::Euronext),
            "XETRA" => Some(Exchange::XETRA),
            "SIX" => Some(Exchange::SIX),
            "BIT" => Some(Exchange::BIT),
            "BME" => Some(Exchange::BME),
            "AEX" => Some(Exchange::AEX),
            "BRU" => Some(Exchange::BRU),
            "LIS" => Some(Exchange::LIS),
            "EPA" => Some(Exchange::EPA),
            "OSL" => Some(Exchange::OSL),
            "STO" => Some(Exchange::STO),
            "CPH" => Some(Exchange::CPH),
            "WSE" => Some(Exchange::WSE),
            "PSE" => Some(Exchange::PSE),
            "BSE" => Some(Exchange::BSE),
            "MOEX" => Some(Exchange::MOEX),
            "BIST" => Some(Exchange::BIST),
            "JSE" => Some(Exchange::JSE),
            "TASE" => Some(Exchange::TASE),
            "BSE_HU" => Some(Exchange::BSE_HU),
            "NSE" => Some(Exchange::NSE),
            "KRX" => Some(Exchange::KRX),
            "SGX" => Some(Exchange::SGX),
            "SET" => Some(Exchange::SET),
            "KLSE" => Some(Exchange::KLSE),
            "PSE_CZ" => Some(Exchange::PSE_CZ),
            "IDX" => Some(Exchange::IDX),
            "HOSE" => Some(Exchange::HOSE),
            _ => Exchange::try_from(s).ok(),
        }
    })
}

/// Convert Exchange to String
#[must_use]
pub fn exchange_to_string(exchange: Option<Exchange>) -> Option<String> {
    exchange.map(|e| e.to_string())
}

/// Convert String to `MarketState` enum
#[must_use]
pub fn string_to_market_state(s: Option<String>) -> Option<MarketState> {
    s.and_then(|s| s.parse().ok())
}

/// Convert `MarketState` to String
#[must_use]
pub fn market_state_to_string(state: Option<MarketState>) -> Option<String> {
    state.map(|s| s.to_string())
}

/// Convert String to `FundKind` enum
#[allow(clippy::single_option_map)]
#[must_use]
pub fn string_to_fund_kind(s: Option<String>) -> Option<FundKind> {
    s.and_then(|s| {
        // Map Yahoo Finance legal types to paft FundKind values
        match s.as_str() {
            "Exchange Traded Fund" => Some(FundKind::Etf),
            "Mutual Fund" => Some(FundKind::MutualFund),
            "Index Fund" => Some(FundKind::IndexFund),
            "Closed-End Fund" => Some(FundKind::ClosedEndFund),
            "Money Market Fund" => Some(FundKind::MoneyMarketFund),
            "Hedge Fund" => Some(FundKind::HedgeFund),
            "Real Estate Investment Trust" => Some(FundKind::Reit),
            "Unit Investment Trust" => Some(FundKind::UnitInvestmentTrust),
            _ => FundKind::try_from(s).ok(),
        }
    })
}

/// Convert `FundKind` to String
#[must_use]
pub fn fund_kind_to_string(kind: Option<FundKind>) -> Option<String> {
    kind.map(|k| k.to_string())
}

/// Convert String to `InsiderPosition` enum
#[must_use]
pub fn string_to_insider_position(s: &str) -> InsiderPosition {
    let token = s.trim();
    let token_nonempty = if token.is_empty() { "UNKNOWN" } else { token };
    token_nonempty.parse().unwrap_or(InsiderPosition::Officer)
}

/// Convert String to `TransactionType` enum
#[must_use]
pub fn string_to_transaction_type(s: &str) -> TransactionType {
    let token = s.trim();
    let token_nonempty = if token.is_empty() { "UNKNOWN" } else { token };
    token_nonempty.parse().unwrap_or(TransactionType::Buy)
}

/// Convert String to Period
#[must_use]
pub fn string_to_period(s: &str) -> Period {
    if s.trim().is_empty() {
        return "UNKNOWN".parse().map_or(Period::Year { year: 1970 }, |p| p);
    }
    s.parse()
        .unwrap_or_else(|_| "UNKNOWN".parse().map_or(Period::Year { year: 1970 }, |p| p))
}

/// Convert String to `RecommendationGrade` enum
#[must_use]
pub fn string_to_recommendation_grade(s: &str) -> RecommendationGrade {
    let token = s.trim();
    let token_nonempty = if token.is_empty() { "UNKNOWN" } else { token };
    token_nonempty.parse().unwrap_or(RecommendationGrade::Hold)
}

/// Convert String to `RecommendationAction` enum
#[must_use]
pub fn string_to_recommendation_action(s: &str) -> RecommendationAction {
    let token = s.trim();
    let token_nonempty = if token.is_empty() { "UNKNOWN" } else { token };
    token_nonempty
        .parse()
        .unwrap_or(RecommendationAction::Maintain)
}
