use httpmock::{Method::GET, Mock, MockServer};
use paft::Decimal;
use paft::fundamentals::statistics::KeyStatistics;
use paft::money::{Currency, IsoCurrency};
use serde_json::Value;
use std::str::FromStr;
use url::Url;
use yfinance_rs::core::conversions::CurrencyValue;
use yfinance_rs::{ProjectionIssue, Ticker, YfClient, YfWarning};

struct CurrencyScaleCase {
    symbol: &'static str,
    quote_currency: &'static str,
    requires_market_cap: bool,
    requires_eps: bool,
    requires_dividend: bool,
}

const RECORDED_CURRENCY_SCALE_CASES: &[CurrencyScaleCase] = &[
    CurrencyScaleCase {
        symbol: "TSCO.L",
        quote_currency: "GBp",
        requires_market_cap: true,
        requires_eps: true,
        requires_dividend: true,
    },
    CurrencyScaleCase {
        symbol: "SBK.JO",
        quote_currency: "ZAc",
        requires_market_cap: true,
        requires_eps: true,
        requires_dividend: true,
    },
    CurrencyScaleCase {
        symbol: "SAP",
        quote_currency: "USD",
        requires_market_cap: true,
        requires_eps: true,
        requires_dividend: true,
    },
    CurrencyScaleCase {
        symbol: "MSFT",
        quote_currency: "USD",
        requires_market_cap: true,
        requires_eps: true,
        requires_dividend: true,
    },
    CurrencyScaleCase {
        symbol: "SPY",
        quote_currency: "USD",
        requires_market_cap: false,
        requires_eps: false,
        requires_dividend: false,
    },
];

fn recorded_quote(symbol: &str) -> (String, Value) {
    let fixture = crate::common::fixture("quote_v7", symbol, "json");
    let raw: Value = serde_json::from_str(&fixture).unwrap();
    (fixture, raw)
}

fn recorded_key_statistics(symbol: &str) -> (String, Value) {
    let fixture = crate::common::fixture(
        crate::common::KEY_STATISTICS_FIXTURE_ENDPOINT,
        symbol,
        "json",
    );
    let raw: Value = serde_json::from_str(&fixture).unwrap();
    (fixture, raw)
}

fn first_quote<'a>(raw: &'a Value, symbol: &str) -> &'a Value {
    raw["quoteResponse"]["result"]
        .as_array()
        .and_then(|quotes| quotes.first())
        .unwrap_or_else(|| panic!("quote_v7 fixture should contain {symbol}"))
}

fn quote_summary_result<'a>(raw: &'a Value, symbol: &str) -> &'a Value {
    raw["quoteSummary"]["result"]
        .as_array()
        .and_then(|results| results.first())
        .unwrap_or_else(|| panic!("key statistics fixture should contain {symbol}"))
}

fn major_currency_code(code: &str) -> &str {
    match code {
        "GBp" | "GBX" => "GBP",
        "ZAc" => "ZAR",
        "ILA" => "ILS",
        _ => code,
    }
}

fn quote_unit_scale(code: &str) -> Decimal {
    match code {
        "GBp" | "GBX" | "ZAc" | "ILA" => Decimal::new(1, 2),
        _ => Decimal::ONE,
    }
}

fn raw_decimal(raw: &Value) -> Option<Decimal> {
    raw.as_number()
        .and_then(|number| Decimal::from_str(&number.to_string()).ok())
        .or_else(|| raw.as_str().and_then(|value| Decimal::from_str(value).ok()))
}

fn raw_summary_decimal(module: &Value, field: &str) -> Option<Decimal> {
    raw_decimal(&module[field]["raw"])
}

fn displayed_percent_points(module: &Value, field: &str) -> Decimal {
    let formatted = module[field]["fmt"]
        .as_str()
        .unwrap_or_else(|| panic!("quoteSummary fixture should contain {field}.fmt"));
    let percent = formatted
        .strip_suffix('%')
        .unwrap_or_else(|| panic!("{field}.fmt should be a percent string, got {formatted:?}"));
    Decimal::from_str(percent)
        .unwrap_or_else(|err| panic!("{field}.fmt should parse as decimal percent points: {err}"))
}

fn assert_decimal_near(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
    let diff = if actual >= expected {
        actual - expected
    } else {
        expected - actual
    };
    assert!(
        diff <= tolerance,
        "{label}: expected {expected} +/- {tolerance}, got {actual}"
    );
}

fn mock_quote_v7_body<'a>(server: &'a MockServer, symbol: &'a str, body: String) -> Mock<'a> {
    server.mock(move |when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", symbol);
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    })
}

fn mock_key_statistics_body<'a>(
    server: &'a MockServer,
    symbol: &'a str,
    crumb: &'a str,
    body: String,
) -> Mock<'a> {
    server.mock(move |when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{symbol}"))
            .query_param("modules", crate::common::KEY_STATISTICS_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    })
}

fn key_statistics_client(server: &MockServer, crumb: &str) -> YfClient {
    YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .build()
        .unwrap()
}

async fn key_statistics_from_bodies(
    symbol: &str,
    quote_body: String,
    key_statistics_body: String,
) -> KeyStatistics {
    let server = MockServer::start();
    let crumb = "test-crumb";
    let quote_mock = mock_quote_v7_body(&server, symbol, quote_body);
    let key_statistics_mock = mock_key_statistics_body(&server, symbol, crumb, key_statistics_body);
    let client = key_statistics_client(&server, crumb);

    let stats = Ticker::new(&client, symbol).key_statistics().await.unwrap();

    quote_mock.assert();
    key_statistics_mock.assert();
    stats
}

fn assert_currency_value<T: CurrencyValue>(
    value: &T,
    expected_amount: Decimal,
    expected_currency: &str,
    symbol: &str,
    field: &str,
) {
    assert_eq!(
        value.currency().to_string(),
        expected_currency,
        "{symbol} {field} currency"
    );
    assert_eq!(value.amount(), expected_amount, "{symbol} {field} amount");
}

fn assert_quote_price<T: CurrencyValue>(
    value: Option<&T>,
    raw: Option<Decimal>,
    currency: &str,
    symbol: &str,
    field: &str,
    required: bool,
) {
    let Some(raw) = raw else {
        assert!(!required, "{symbol} fixture missing {field}");
        return;
    };
    let value = value.unwrap_or_else(|| panic!("{symbol} {field} should map"));
    assert_currency_value(
        value,
        raw * quote_unit_scale(currency),
        major_currency_code(currency),
        symbol,
        field,
    );
}

fn assert_major_price<T: CurrencyValue>(
    value: Option<&T>,
    raw: Option<Decimal>,
    currency: &str,
    symbol: &str,
    field: &str,
    required: bool,
) {
    let Some(raw) = raw else {
        assert!(!required, "{symbol} fixture missing {field}");
        return;
    };
    let value = value.unwrap_or_else(|| panic!("{symbol} {field} should map"));
    assert_currency_value(value, raw, major_currency_code(currency), symbol, field);
}

fn assert_major_money<T: CurrencyValue>(
    value: Option<&T>,
    raw: Option<Decimal>,
    currency: &str,
    symbol: &str,
    field: &str,
    required: bool,
) {
    let Some(raw) = raw else {
        assert!(!required, "{symbol} fixture missing {field}");
        return;
    };
    let value = value.unwrap_or_else(|| panic!("{symbol} {field} should map"));
    assert_eq!(
        value.currency().to_string(),
        major_currency_code(currency),
        "{symbol} {field} currency"
    );
    assert_eq!(value.amount(), raw, "{symbol} {field} amount");
}

fn assert_v7_key_statistics(stats: &KeyStatistics, raw_quote: &serde_json::Value) {
    assert_eq!(
        stats.shares_outstanding,
        raw_quote["sharesOutstanding"].as_u64()
    );
    assert_eq!(
        stats.average_daily_volume_3m,
        raw_quote["averageDailyVolume3Month"].as_u64()
    );
    assert_eq!(
        stats.market_cap.as_ref().unwrap().amount(),
        raw_decimal(&raw_quote["marketCap"]).unwrap()
    );
    assert_eq!(
        stats.eps_trailing_twelve_months.as_ref().unwrap().amount(),
        raw_decimal(&raw_quote["epsTrailingTwelveMonths"]).unwrap()
    );
    assert_eq!(
        stats.dividend_per_share_forward.as_ref().unwrap().amount(),
        raw_decimal(&raw_quote["dividendRate"]).unwrap()
    );
    assert_eq!(
        stats.fifty_two_week_high.as_ref().unwrap().amount(),
        raw_decimal(&raw_quote["fiftyTwoWeekHigh"]).unwrap()
    );
    assert_eq!(
        stats.fifty_two_week_low.as_ref().unwrap().amount(),
        raw_decimal(&raw_quote["fiftyTwoWeekLow"]).unwrap()
    );
    assert_eq!(
        stats.pe_trailing_twelve_months,
        raw_decimal(&raw_quote["trailingPE"])
    );
    assert_eq!(
        stats.dividend_yield_trailing,
        raw_decimal(&raw_quote["trailingAnnualDividendYield"])
    );
    assert_eq!(
        stats.dividend_yield_forward,
        raw_decimal(&raw_quote["dividendYield"]).map(|value| value / paft::Decimal::from(100))
    );
}

fn assert_quote_summary_backfilled_statistics(stats: &KeyStatistics, fixture: &str) {
    let raw: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let quote_summary = raw["quoteSummary"]["result"]
        .as_array()
        .and_then(|results| results.first())
        .expect("quoteSummary fixture should contain MSFT");
    let summary_detail = &quote_summary["summaryDetail"];
    let default_key_statistics = &quote_summary["defaultKeyStatistics"];

    assert_quote_summary_valuation_fields(stats, summary_detail, default_key_statistics);
    assert_quote_summary_dividend_fields(stats, summary_detail, fixture);
    assert_quote_summary_range_fields(stats, summary_detail);
    assert_eq!(
        stats.average_daily_volume_3m,
        summary_detail["averageVolume"]["raw"].as_u64()
    );
    assert_eq!(stats.beta, Some(crate::common::quote_summary_beta(fixture)));
}

fn assert_quote_summary_valuation_fields(
    stats: &KeyStatistics,
    summary_detail: &serde_json::Value,
    default_key_statistics: &serde_json::Value,
) {
    assert_eq!(
        stats.market_cap.as_ref().unwrap().amount(),
        paft::Decimal::from(summary_detail["marketCap"]["raw"].as_i64().unwrap())
    );
    assert_eq!(
        stats.market_cap.as_ref().unwrap().currency(),
        &Currency::Iso(IsoCurrency::USD)
    );
    assert_eq!(
        stats.shares_outstanding,
        default_key_statistics["sharesOutstanding"]["raw"].as_u64()
    );
    assert_eq!(
        stats.eps_trailing_twelve_months.as_ref().unwrap().amount(),
        raw_summary_decimal(default_key_statistics, "trailingEps").unwrap()
    );
    assert_eq!(
        stats.pe_trailing_twelve_months,
        raw_summary_decimal(summary_detail, "trailingPE")
    );
}

fn assert_quote_summary_dividend_fields(
    stats: &KeyStatistics,
    summary_detail: &serde_json::Value,
    fixture: &str,
) {
    assert_eq!(
        stats.dividend_per_share_forward.as_ref().unwrap().amount(),
        raw_summary_decimal(summary_detail, "dividendRate").unwrap()
    );
    assert_eq!(
        stats.dividend_yield_trailing,
        raw_summary_decimal(summary_detail, "trailingAnnualDividendYield")
    );
    assert_eq!(
        stats.dividend_yield_forward,
        raw_summary_decimal(summary_detail, "dividendYield")
    );
    assert_eq!(
        stats.ex_dividend_date,
        Some(crate::common::quote_summary_ex_dividend_date(fixture))
    );
}

fn assert_quote_summary_range_fields(stats: &KeyStatistics, summary_detail: &serde_json::Value) {
    assert_eq!(
        stats.fifty_two_week_high.as_ref().unwrap().amount(),
        raw_summary_decimal(summary_detail, "fiftyTwoWeekHigh").unwrap()
    );
    assert_eq!(
        stats.fifty_two_week_low.as_ref().unwrap().amount(),
        raw_summary_decimal(summary_detail, "fiftyTwoWeekLow").unwrap()
    );
}

#[tokio::test]
async fn key_statistics_market_cap_uses_major_units_for_minor_unit_quote_currency() {
    let server = MockServer::start();
    let sym = "TSCO.L";
    let crumb = "test-crumb";
    let quote_body = r#"{
      "quoteResponse": {
        "result": [{
          "symbol": "TSCO.L",
          "quoteType": "EQUITY",
          "currency": "GBp",
          "financialCurrency": "GBP",
          "regularMarketPrice": 444.1,
          "fiftyTwoWeekHigh": 455.0,
          "fiftyTwoWeekLow": 300.0,
          "epsTrailingTwelveMonths": 0.27,
          "dividendRate": 0.15,
          "marketCap": 28024838144,
          "sharesOutstanding": 6310478847
        }],
        "error": null
      }
    }"#;

    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(quote_body);
    });
    let key_statistics_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", crate::common::KEY_STATISTICS_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":[{}],"error":null}}"#);
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .build()
        .unwrap();

    let stats = Ticker::new(&client, sym).key_statistics().await.unwrap();

    quote_mock.assert();
    key_statistics_mock.assert();
    let market_cap = stats.market_cap.as_ref().expect("market cap should map");
    assert_eq!(market_cap.currency(), &Currency::Iso(IsoCurrency::GBP));
    assert_eq!(market_cap.amount(), Decimal::from(28_024_838_144_u64));

    let eps = stats
        .eps_trailing_twelve_months
        .as_ref()
        .expect("EPS should map");
    assert_eq!(eps.currency(), &Currency::Iso(IsoCurrency::GBP));
    assert_eq!(eps.amount(), Decimal::new(27, 2));

    let dividend = stats
        .dividend_per_share_forward
        .as_ref()
        .expect("dividend should map");
    assert_eq!(dividend.currency(), &Currency::Iso(IsoCurrency::GBP));
    assert_eq!(dividend.amount(), Decimal::new(15, 2));

    let high = stats
        .fifty_two_week_high
        .as_ref()
        .expect("52-week high should map");
    assert_eq!(high.currency(), &Currency::Iso(IsoCurrency::GBP));
    assert_eq!(high.amount(), Decimal::new(455, 2));
}

#[tokio::test]
async fn key_statistics_with_diagnostics_reports_invalid_financial_currency() {
    let server = MockServer::start();
    let sym = "AAPL";
    let crumb = "test-crumb";
    let quote_body = r#"{
      "quoteResponse": {
        "result": [{
          "symbol": "AAPL",
          "quoteType": "EQUITY",
          "currency": "USD",
          "financialCurrency": "!!!",
          "regularMarketPrice": 190.25,
          "epsTrailingTwelveMonths": 6.43,
          "dividendRate": 1.04,
          "marketCap": 3000000000000
        }],
        "error": null
      }
    }"#;

    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(quote_body);
    });
    let key_statistics_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", crate::common::KEY_STATISTICS_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":[{}],"error":null}}"#);
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .build()
        .unwrap();

    let response = Ticker::new(&client, sym)
        .key_statistics_with_diagnostics()
        .await
        .unwrap();

    quote_mock.assert();
    key_statistics_mock.assert();
    assert!(response.data.market_cap.is_some());
    assert!(response.data.eps_trailing_twelve_months.is_none());
    assert!(response.diagnostics.warnings.iter().any(|warning| {
        matches!(
            warning,
            YfWarning::OmittedPresentField {
                endpoint: "key_statistics",
                path: "epsTrailingTwelveMonths",
                key: Some(key),
                reason: ProjectionIssue::InvalidCurrency { code },
            } if key == sym && code == "!!!"
        )
    }));
}

#[tokio::test]
async fn key_statistics_with_diagnostics_omits_malformed_v7_market_cap() {
    let server = MockServer::start();
    let sym = "AAPL";
    let crumb = "test-crumb";
    let quote_body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol": "AAPL",
            "quoteType": "EQUITY",
            "currency": "USD",
            "marketCap": "not-a-number",
            "trailingPE": 30.5
          }
        ],
        "error": null
      }
    }"#;

    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(quote_body);
    });
    let key_statistics_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", crate::common::KEY_STATISTICS_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":[{}],"error":null}}"#);
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .build()
        .unwrap();

    let response = Ticker::new(&client, sym)
        .key_statistics_with_diagnostics()
        .await
        .unwrap();

    quote_mock.assert();
    key_statistics_mock.assert();
    assert!(response.data.market_cap.is_none());
    assert_eq!(
        response.data.pe_trailing_twelve_months,
        Some(paft::Decimal::new(305, 1))
    );
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            endpoint: "key_statistics",
            path: "marketCap",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "marketCap",
                ..
            },
        } if key == sym
    )));
}

#[tokio::test]
async fn key_statistics_market_cap_preserves_large_integer_precision() {
    let server = MockServer::start();
    let sym = "BIG";
    let crumb = "test-crumb";
    let exact = 9_007_199_254_740_993_i64;
    let quote_body = format!(
        r#"{{
      "quoteResponse": {{
        "result": [{{
          "symbol": "{sym}",
          "quoteType": "EQUITY",
          "currency": "USD",
          "regularMarketPrice": 10.0,
          "marketCap": {exact}
        }}],
        "error": null
      }}
    }}"#
    );

    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(quote_body);
    });
    let key_statistics_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", crate::common::KEY_STATISTICS_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":[{}],"error":null}}"#);
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .build()
        .unwrap();

    let stats = Ticker::new(&client, sym).key_statistics().await.unwrap();

    quote_mock.assert();
    key_statistics_mock.assert();
    let market_cap = stats.market_cap.as_ref().expect("market cap should map");
    assert_eq!(market_cap.amount(), paft::Decimal::from(exact));
    assert_eq!(market_cap.currency(), &Currency::Iso(IsoCurrency::USD));
}

#[tokio::test]
async fn key_statistics_v7_dividend_yield_units_are_fixture_locked() {
    let server = MockServer::start();
    let sym = "MSFT";
    let crumb = "test-crumb";
    let (fixture, raw_fixture) = recorded_quote(sym);
    let raw_quote = first_quote(&raw_fixture, sym);
    let trailing_raw = raw_decimal(&raw_quote["trailingAnnualDividendYield"]).unwrap();
    let forward_raw = raw_decimal(&raw_quote["dividendYield"]).unwrap();
    assert!(
        forward_raw > Decimal::new(1, 2),
        "fixture should keep v7 dividendYield in percent points"
    );
    assert!(
        trailing_raw < Decimal::new(1, 1),
        "fixture should keep v7 trailingAnnualDividendYield as a fraction"
    );

    let quote_mock = mock_quote_v7_body(&server, sym, fixture);
    let key_statistics_mock = mock_key_statistics_body(
        &server,
        sym,
        crumb,
        r#"{"quoteSummary":{"result":[{}],"error":null}}"#.to_string(),
    );
    let client = key_statistics_client(&server, crumb);

    let stats = Ticker::new(&client, sym).key_statistics().await.unwrap();

    quote_mock.assert();
    key_statistics_mock.assert();
    assert_eq!(stats.dividend_yield_trailing, Some(trailing_raw));
    assert_eq!(
        stats.dividend_yield_forward,
        Some(forward_raw / Decimal::from(100))
    );
    assert_ne!(
        stats.dividend_yield_trailing,
        Some(trailing_raw / Decimal::from(100)),
        "trailingAnnualDividendYield must not be divided a second time"
    );
    assert_ne!(
        stats.dividend_yield_forward,
        Some(forward_raw),
        "dividendYield must not be kept as raw percent points"
    );
}

#[tokio::test]
async fn aapl_dividend_yield_conventions_reconcile_across_recorded_paths() {
    let sym = "AAPL";
    let (quote_fixture, raw_quote_fixture) = recorded_quote(sym);
    let (key_statistics_fixture, raw_key_statistics_fixture) = recorded_key_statistics(sym);
    let raw_quote = first_quote(&raw_quote_fixture, sym);
    let quote_summary = quote_summary_result(&raw_key_statistics_fixture, sym);
    let summary_detail = &quote_summary["summaryDetail"];

    let v7_percent_points = raw_decimal(&raw_quote["dividendYield"])
        .expect("quote_v7 fixture should contain dividendYield");
    let quote_summary_fraction = raw_summary_decimal(summary_detail, "dividendYield")
        .expect("quoteSummary fixture should contain summaryDetail.dividendYield.raw");
    let displayed_fraction =
        displayed_percent_points(summary_detail, "dividendYield") / Decimal::from(100);
    let yield_tolerance = Decimal::new(1, 8);

    assert!(
        v7_percent_points > Decimal::new(1, 2),
        "quote_v7 dividendYield should be recorded as displayed percent points"
    );
    assert!(
        quote_summary_fraction < Decimal::new(1, 1),
        "quoteSummary summaryDetail.dividendYield raw should be recorded as a fraction"
    );
    assert_decimal_near(
        v7_percent_points / Decimal::from(100),
        displayed_fraction,
        yield_tolerance,
        "v7 dividendYield",
    );
    assert_decimal_near(
        quote_summary_fraction,
        displayed_fraction,
        yield_tolerance,
        "quoteSummary dividendYield",
    );

    let server = MockServer::start();
    let quote_mock = mock_quote_v7_body(&server, sym, quote_fixture.clone());
    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();
    let quote = Ticker::new(&client, sym).quote().await.unwrap();
    quote_mock.assert();
    assert_eq!(quote.instrument.symbol.as_str(), sym);

    let v7_only_stats = key_statistics_from_bodies(
        sym,
        quote_fixture.clone(),
        r#"{"quoteSummary":{"result":[{}],"error":null}}"#.to_string(),
    )
    .await;

    let quote_summary_only_quote = format!(
        r#"{{
          "quoteResponse": {{
            "result": [{{
              "symbol": "{sym}",
              "quoteType": "EQUITY",
              "currency": "USD"
            }}],
            "error": null
          }}
        }}"#
    );
    let quote_summary_only_stats = key_statistics_from_bodies(
        sym,
        quote_summary_only_quote,
        key_statistics_fixture.clone(),
    )
    .await;
    let merged_stats = key_statistics_from_bodies(sym, quote_fixture, key_statistics_fixture).await;

    assert_decimal_near(
        v7_only_stats
            .dividend_yield_forward
            .expect("v7 path should map dividend_yield_forward"),
        displayed_fraction,
        yield_tolerance,
        "v7 mapped dividend_yield_forward",
    );
    assert_decimal_near(
        quote_summary_only_stats
            .dividend_yield_forward
            .expect("quoteSummary path should map dividend_yield_forward"),
        displayed_fraction,
        yield_tolerance,
        "quoteSummary mapped dividend_yield_forward",
    );
    assert_decimal_near(
        merged_stats
            .dividend_yield_forward
            .expect("merged path should map dividend_yield_forward"),
        displayed_fraction,
        yield_tolerance,
        "merged dividend_yield_forward",
    );
}

#[tokio::test]
async fn key_statistics_recorded_v7_currency_units_are_field_scoped() {
    for case in RECORDED_CURRENCY_SCALE_CASES {
        let server = MockServer::start();
        let crumb = "test-crumb";
        let (quote_fixture, raw_quote_fixture) = recorded_quote(case.symbol);
        let (key_statistics_fixture, _) = recorded_key_statistics(case.symbol);
        let raw_quote = first_quote(&raw_quote_fixture, case.symbol);
        let quote_currency = raw_quote["currency"]
            .as_str()
            .unwrap_or_else(|| panic!("{} quote fixture missing currency", case.symbol));
        let financial_currency = raw_quote["financialCurrency"]
            .as_str()
            .unwrap_or(quote_currency);
        assert_eq!(quote_currency, case.quote_currency);

        let quote_mock = mock_quote_v7_body(&server, case.symbol, quote_fixture);
        let key_statistics_mock =
            mock_key_statistics_body(&server, case.symbol, crumb, key_statistics_fixture);
        let client = key_statistics_client(&server, crumb);

        let stats = Ticker::new(&client, case.symbol)
            .key_statistics()
            .await
            .unwrap();

        quote_mock.assert();
        key_statistics_mock.assert();

        assert_major_money(
            stats.market_cap.as_ref(),
            raw_decimal(&raw_quote["marketCap"]),
            quote_currency,
            case.symbol,
            "marketCap",
            case.requires_market_cap,
        );
        assert_major_price(
            stats.eps_trailing_twelve_months.as_ref(),
            raw_decimal(&raw_quote["epsTrailingTwelveMonths"]),
            financial_currency,
            case.symbol,
            "epsTrailingTwelveMonths",
            case.requires_eps,
        );
        assert_major_price(
            stats.dividend_per_share_forward.as_ref(),
            raw_decimal(&raw_quote["dividendRate"]),
            quote_currency,
            case.symbol,
            "dividendRate",
            case.requires_dividend,
        );
        assert_quote_price(
            stats.fifty_two_week_high.as_ref(),
            raw_decimal(&raw_quote["fiftyTwoWeekHigh"]),
            quote_currency,
            case.symbol,
            "fiftyTwoWeekHigh",
            true,
        );
        assert_quote_price(
            stats.fifty_two_week_low.as_ref(),
            raw_decimal(&raw_quote["fiftyTwoWeekLow"]),
            quote_currency,
            case.symbol,
            "fiftyTwoWeekLow",
            true,
        );
    }
}

#[tokio::test]
async fn key_statistics_recorded_quote_summary_currency_units_are_field_scoped() {
    for case in RECORDED_CURRENCY_SCALE_CASES {
        let server = MockServer::start();
        let crumb = "test-crumb";
        let (_, raw_quote_fixture) = recorded_quote(case.symbol);
        let raw_quote = first_quote(&raw_quote_fixture, case.symbol);
        let quote_type = raw_quote["quoteType"]
            .as_str()
            .unwrap_or_else(|| panic!("{} quote fixture missing quoteType", case.symbol));
        let quote_currency = raw_quote["currency"]
            .as_str()
            .unwrap_or_else(|| panic!("{} quote fixture missing currency", case.symbol));
        assert_eq!(quote_currency, case.quote_currency);

        let (key_statistics_fixture, raw_key_statistics_fixture) =
            recorded_key_statistics(case.symbol);
        let quote_summary = quote_summary_result(&raw_key_statistics_fixture, case.symbol);
        let summary_detail = &quote_summary["summaryDetail"];
        let default_key_statistics = &quote_summary["defaultKeyStatistics"];
        let summary_currency = summary_detail["currency"]
            .as_str()
            .unwrap_or_else(|| panic!("{} key statistics fixture missing currency", case.symbol));

        let quote_body = serde_json::json!({
            "quoteResponse": {
                "result": [{
                    "symbol": case.symbol,
                    "quoteType": quote_type,
                    "currency": quote_currency,
                    "financialCurrency": raw_quote["financialCurrency"].as_str()
                }],
                "error": null
            }
        })
        .to_string();

        let quote_mock = mock_quote_v7_body(&server, case.symbol, quote_body);
        let key_statistics_mock =
            mock_key_statistics_body(&server, case.symbol, crumb, key_statistics_fixture);
        let client = key_statistics_client(&server, crumb);

        let stats = Ticker::new(&client, case.symbol)
            .key_statistics()
            .await
            .unwrap();

        quote_mock.assert();
        key_statistics_mock.assert();

        assert_major_money(
            stats.market_cap.as_ref(),
            raw_summary_decimal(summary_detail, "marketCap"),
            summary_currency,
            case.symbol,
            "summaryDetail.marketCap",
            case.requires_market_cap,
        );
        assert_major_price(
            stats.eps_trailing_twelve_months.as_ref(),
            raw_summary_decimal(default_key_statistics, "trailingEps"),
            summary_currency,
            case.symbol,
            "defaultKeyStatistics.trailingEps",
            case.requires_eps,
        );
        assert_major_price(
            stats.dividend_per_share_forward.as_ref(),
            raw_summary_decimal(summary_detail, "dividendRate"),
            summary_currency,
            case.symbol,
            "summaryDetail.dividendRate",
            case.requires_dividend,
        );
        assert_quote_price(
            stats.fifty_two_week_high.as_ref(),
            raw_summary_decimal(summary_detail, "fiftyTwoWeekHigh"),
            summary_currency,
            case.symbol,
            "summaryDetail.fiftyTwoWeekHigh",
            true,
        );
        assert_quote_price(
            stats.fifty_two_week_low.as_ref(),
            raw_summary_decimal(summary_detail, "fiftyTwoWeekLow"),
            summary_currency,
            case.symbol,
            "summaryDetail.fiftyTwoWeekLow",
            true,
        );
    }
}

#[tokio::test]
async fn key_statistics_merge_v7_quote_and_quote_summary_fixtures() {
    let server = MockServer::start();
    let sym = "MSFT";
    let crumb = "test-crumb";
    let fixture = crate::common::fixture("quote_v7", sym, "json");
    let key_statistics_fixture =
        crate::common::fixture(crate::common::KEY_STATISTICS_FIXTURE_ENDPOINT, sym, "json");
    let raw: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let raw_quote = raw["quoteResponse"]["result"]
        .as_array()
        .and_then(|quotes| quotes.first())
        .expect("quote fixture should contain MSFT");
    assert!(
        raw_quote.get("beta").is_none(),
        "v7 quote fixture must not drive the beta assertion"
    );

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture);
    });
    let key_statistics_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", crate::common::KEY_STATISTICS_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(key_statistics_fixture.clone());
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, sym);

    let stats = ticker.key_statistics().await.unwrap();
    mock.assert();
    key_statistics_mock.assert();

    assert_v7_key_statistics(&stats, raw_quote);
    assert_eq!(
        stats.beta,
        Some(crate::common::quote_summary_beta(&key_statistics_fixture))
    );
    assert_eq!(
        stats.ex_dividend_date,
        Some(crate::common::quote_summary_ex_dividend_date(
            &key_statistics_fixture
        ))
    );

    #[cfg(feature = "dataframe")]
    {
        use yfinance_rs::ToDataFrame;

        let df = stats.to_dataframe().unwrap();
        assert_eq!(df.height(), 1);
    }
}

#[tokio::test]
async fn key_statistics_backfills_recorded_quote_summary_when_v7_omits_fields() {
    let server = MockServer::start();
    let sym = "MSFT";
    let crumb = "test-crumb";
    let key_statistics_fixture =
        crate::common::fixture(crate::common::KEY_STATISTICS_FIXTURE_ENDPOINT, sym, "json");
    let quote_body = r#"{
      "quoteResponse": {
        "result": [{
          "symbol": "MSFT",
          "quoteType": "EQUITY",
          "currency": "USD"
        }],
        "error": null
      }
    }"#;

    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(quote_body);
    });
    let key_statistics_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", crate::common::KEY_STATISTICS_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(key_statistics_fixture.clone());
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .build()
        .unwrap();

    let stats = Ticker::new(&client, sym).key_statistics().await.unwrap();

    quote_mock.assert();
    key_statistics_mock.assert();
    assert_quote_summary_backfilled_statistics(&stats, &key_statistics_fixture);
}
