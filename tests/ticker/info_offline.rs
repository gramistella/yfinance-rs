use httpmock::{Method::GET, Mock, MockServer};
use paft::Decimal;
use paft::fundamentals::profile::Profile;
use std::time::Duration;
use url::Url;
use yfinance_rs::{
    ProjectionIssue, Ticker, YfClient, YfWarning,
    core::client::{Backoff, CacheMode, RetryConfig},
};

const INFO_QUOTE_SUMMARY_MODULES: &str = "summaryDetail,defaultKeyStatistics,assetProfile,quoteType,fundProfile,financialData,recommendationTrend,calendarEvents";

fn json_decimal(value: &serde_json::Value) -> Decimal {
    value
        .as_number()
        .expect("fixture decimal")
        .to_string()
        .parse()
        .expect("valid fixture decimal")
}

fn mock_info_quote<'a>(server: &'a MockServer, sym: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("quote_v7", sym, "json"));
    })
}

fn merged_quote_summary_fixture(sym: &str, endpoints: &[&str]) -> String {
    let mut merged = serde_json::Map::new();
    for endpoint in endpoints {
        let raw: serde_json::Value =
            serde_json::from_str(&crate::common::fixture(endpoint, sym, "json")).unwrap();
        let result = raw["quoteSummary"]["result"]
            .as_array()
            .and_then(|results| results.first())
            .and_then(serde_json::Value::as_object)
            .unwrap_or_else(|| panic!("{endpoint} fixture should contain quoteSummary.result[0]"));

        for (key, value) in result {
            merged.insert(key.clone(), value.clone());
        }
    }

    serde_json::json!({
        "quoteSummary": {
            "result": [merged],
            "error": null
        }
    })
    .to_string()
}

#[tokio::test]
async fn offline_info_uses_recorded_fixtures() {
    let server = MockServer::start();
    let sym = "MSFT";
    let crumb = "test-crumb";
    let quote_fixture = crate::common::fixture("quote_v7", sym, "json");
    let key_statistics_fixture =
        crate::common::fixture(crate::common::KEY_STATISTICS_FIXTURE_ENDPOINT, sym, "json");
    let info_quote_summary_fixture = merged_quote_summary_fixture(
        sym,
        &[
            crate::common::KEY_STATISTICS_FIXTURE_ENDPOINT,
            "profile_api_assetProfile-quoteType-fundProfile",
            "analysis_api_recommendationTrend-financialData",
            "fundamentals_api_calendarEvents",
        ],
    );
    let raw_quote_fixture: serde_json::Value = serde_json::from_str(&quote_fixture).unwrap();
    let raw_quote = raw_quote_fixture["quoteResponse"]["result"]
        .as_array()
        .and_then(|quotes| quotes.first())
        .expect("quote fixture should contain MSFT");

    // 1. Mock for quote::fetch_quote -> uses `quote_v7_MSFT.json`
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(quote_fixture);
    });

    let info_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INFO_QUOTE_SUMMARY_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(info_quote_summary_fixture);
    });
    let esg_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "esgScores")
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("esg_api_esgScores", sym, "json"));
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
    let info = ticker.info().await.unwrap();

    // Assert all mocks were hit
    quote_mock.assert_calls(1);
    info_mock.assert();
    esg_mock.assert_calls(0);

    // Verify data aggregation with more robust checks. Run recorders if these fail.
    assert_eq!(info.snapshot.instrument.symbol.as_str(), "MSFT");
    assert!(
        info.snapshot.last.is_some(),
        "Price missing from quote fixture."
    );
    assert!(info.profile.is_some());
    assert_eq!(
        info.key_statistics.shares_outstanding,
        raw_quote["sharesOutstanding"].as_u64()
    );
    assert_eq!(
        info.key_statistics.beta,
        Some(crate::common::quote_summary_beta(&key_statistics_fixture))
    );
    assert_eq!(
        info.key_statistics.ex_dividend_date,
        Some(crate::common::quote_summary_ex_dividend_date(
            &key_statistics_fixture
        ))
    );
    assert_eq!(
        info.moving_averages.fifty_day.as_ref().unwrap().amount(),
        json_decimal(&raw_quote["fiftyDayAverage"])
    );
    assert_eq!(
        info.moving_averages
            .two_hundred_day
            .as_ref()
            .unwrap()
            .amount(),
        json_decimal(&raw_quote["twoHundredDayAverage"])
    );
    assert!(
        info.calendar
            .and_then(|calendar| calendar.dividend_payment_date)
            .is_some(),
        "dividend payment date should fall back to v7 quote dividendDate"
    );
}

#[tokio::test]
async fn ticker_info_missing_optional_quote_summary_modules_are_none() {
    let server = MockServer::start();
    let sym = "MSFT";
    let crumb = "crumb";
    let quote_mock = mock_info_quote(&server, sym);
    let info_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INFO_QUOTE_SUMMARY_MODULES)
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
        .retry_enabled(false)
        .build()
        .unwrap();

    let response = Ticker::new(&client, sym)
        .info_with_diagnostics()
        .await
        .unwrap();

    quote_mock.assert();
    info_mock.assert();
    assert!(response.data.price_target.is_none());
    assert!(response.data.recommendation_summary.is_none());
    assert!(
        response
            .data
            .calendar
            .and_then(|calendar| calendar.dividend_payment_date)
            .is_some(),
        "missing calendarEvents should not block the v7 quote dividendDate fallback"
    );
    for feature in ["financialData", "recommendationTrend", "calendarEvents"] {
        assert!(
            response.diagnostics.warnings.iter().any(|warning| matches!(
                warning,
                YfWarning::ProviderFeatureUnavailable {
                    feature: actual,
                    reason: ProjectionIssue::ProviderUnavailable { .. },
                    ..
                } if *actual == feature
            )),
            "expected unavailable-feature diagnostic for {feature}"
        );
    }
}

#[tokio::test]
async fn ticker_info_backfills_moving_averages_from_summary_detail() {
    let server = MockServer::start();
    let sym = "AAPL";
    let crumb = "crumb";
    let quote_body = serde_json::json!({
        "quoteResponse": {
            "result": [{
                "symbol": sym,
                "quoteType": "EQUITY",
                "regularMarketPrice": 195.0,
                "regularMarketPreviousClose": 194.0,
                "currency": "USD",
                "fullExchangeName": "NasdaqGS"
            }],
            "error": null
        }
    })
    .to_string();

    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(quote_body);
    });

    let info_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INFO_QUOTE_SUMMARY_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(
                serde_json::json!({
                    "quoteSummary": {
                        "result": [{
                            "summaryDetail": {
                                "currency": "USD",
                                "fiftyDayAverage": { "raw": 190.25, "fmt": "190.25" },
                                "twoHundredDayAverage": { "raw": 180.75, "fmt": "180.75" }
                            }
                        }],
                        "error": null
                    }
                })
                .to_string(),
            );
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .retry_enabled(false)
        .build()
        .unwrap();

    let info = Ticker::new(&client, sym).info().await.unwrap();

    quote_mock.assert();
    info_mock.assert();
    assert_eq!(
        info.moving_averages.fifty_day.as_ref().unwrap().amount(),
        Decimal::new(19025, 2)
    );
    assert_eq!(
        info.moving_averages
            .two_hundred_day
            .as_ref()
            .unwrap()
            .amount(),
        Decimal::new(18075, 2)
    );
}

#[tokio::test]
async fn ticker_info_synthetic_etf_missing_legal_type_keeps_profile() {
    let server = MockServer::start();
    let sym = "SPY";
    let crumb = "crumb";
    let _quote_mock = mock_info_quote(&server, sym);
    let info_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INFO_QUOTE_SUMMARY_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(
                serde_json::json!({
                    "quoteSummary": {
                        "error": null,
                        "result": [{
                            "fundProfile": {
                                "family": "State Street Investment Management"
                            },
                            "quoteType": {
                                "exchange": "PCX",
                                "quoteType": "ETF",
                                "longName": "State Street SPDR S&P 500 ETF Trust",
                                "symbol": sym
                            }
                        }]
                    }
                })
                .to_string(),
            );
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .retry_enabled(false)
        .build()
        .unwrap();

    let info = Ticker::new(&client, sym).info().await.unwrap();

    info_mock.assert();
    match info.profile {
        Some(Profile::Fund(fund)) => {
            assert_eq!(fund.name, "State Street SPDR S&P 500 ETF Trust");
            assert_eq!(
                fund.family.as_deref(),
                Some("State Street Investment Management")
            );
            assert_eq!(fund.kind.to_string(), "ETF");
        }
        other => panic!("expected ETF profile, got {other:?}"),
    }
}

#[tokio::test]
async fn info_with_diagnostics_does_not_fetch_dead_esg_module() {
    let server = MockServer::start();
    let sym = "MSFT";
    let crumb = "crumb";
    let _quote_mock = mock_info_quote(&server, sym);
    let info_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INFO_QUOTE_SUMMARY_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":[{}],"error":null}}"#);
    });
    let esg_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "esgScores")
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
        .retry_enabled(false)
        .build()
        .unwrap();

    Ticker::new(&client, sym)
        .info_with_diagnostics()
        .await
        .unwrap();

    info_mock.assert();
    esg_mock.assert_calls(0);
}

#[tokio::test]
async fn ticker_info_profile_respects_ticker_cache_bypass() {
    let server = MockServer::start();
    let sym = "AAPL";
    let crumb = "crumb";
    let _quote_mock = mock_info_quote(&server, sym);
    let info_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INFO_QUOTE_SUMMARY_MODULES)
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(merged_quote_summary_fixture(
                sym,
                &["profile_api_assetProfile-quoteType-fundProfile"],
            ));
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .cache_ttl(Duration::from_mins(1))
        .retry_enabled(false)
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, sym).cache_mode(CacheMode::Bypass);

    assert!(ticker.info().await.unwrap().profile.is_some());
    assert!(ticker.info().await.unwrap().profile.is_some());
    info_mock.assert_calls(2);
}

#[tokio::test]
async fn ticker_info_profile_respects_ticker_retry_policy() {
    let server = MockServer::start();
    let sym = "AAPL";
    let crumb = "crumb";
    let _quote_mock = mock_info_quote(&server, sym);
    let info_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INFO_QUOTE_SUMMARY_MODULES)
            .query_param("crumb", crumb);
        then.status(503).body("Service Unavailable");
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", crumb)
        .retry_enabled(false)
        .build()
        .unwrap();
    let max_retries = 2;
    let ticker_retry = RetryConfig {
        backoff: Backoff::Fixed(Duration::from_millis(1)),
        max_retries,
        ..RetryConfig::default()
    };

    let ticker = Ticker::new(&client, sym).retry_policy(Some(ticker_retry));
    let info = ticker.info().await.unwrap();

    info_mock.assert_calls((1 + max_retries) as usize);
    assert!(info.profile.is_none());
}
