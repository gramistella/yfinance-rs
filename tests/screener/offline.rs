use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use paft::Decimal;
use serde_json::json;
use std::time::Duration;
use url::Url;
use yfinance_rs::{
    CacheMode, EquityQuery, PredefinedScreener, ProjectionIssue, Region, ScreenerBuilder,
    YahooExchangeCode, YfClient, YfError, YfWarning, equity_fields,
};

fn fixture(endpoint: &str, key: &str) -> String {
    crate::common::fixture(endpoint, key, "json")
}

#[tokio::test]
async fn offline_predefined_day_gainers_uses_get_with_expected_params() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("screener_predefined", "day_gainers"));
    });

    let client = YfClient::default();
    let base = Url::parse(&format!(
        "{}/v1/finance/screener/predefined/saved",
        server.base_url()
    ))
    .unwrap();
    let response = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base)
        .cache_mode(CacheMode::Bypass)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    assert!(response.count.is_some_and(|count| count > 0));
    assert!(!response.results.is_empty());
    assert!(response.results[0].symbol.is_some());
    assert!(response.results[0].quote_type.is_some());
}

#[tokio::test]
async fn predefined_screener_does_not_cache_successful_status_api_error_body() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                    "finance": {
                        "result": null,
                        "error": {
                            "code": "Unauthorized",
                            "description": "Invalid crumb"
                        }
                    }
                }"#,
            );
    });

    let client = YfClient::builder()
        .cache_ttl(Duration::from_mins(1))
        .build()
        .unwrap();
    let base = Url::parse(&format!(
        "{}/v1/finance/screener/predefined/saved",
        server.base_url()
    ))
    .unwrap();

    for _ in 0..2 {
        let err = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
            .predefined_screener_base(base.clone())
            .cache_mode(CacheMode::Use)
            .fetch()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            YfError::Api(message) if message.contains("Unauthorized")
        ));
    }

    mock.assert_calls(2);
}

#[tokio::test]
async fn predefined_screener_preserves_exact_financial_decimals() {
    let server = MockServer::start();
    let exact = 9_007_199_254_740_993_i64;
    let body = format!(
        r#"{{
      "finance": {{
        "error": null,
        "result": [{{
          "count": 1.0,
          "quotes": [{{
            "symbol": "BIG",
            "quoteType": "EQUITY",
            "shortName": "Big Inc.",
            "regularMarketPrice": 1234567890.123456789012345678,
            "regularMarketChangePercent": 0.031600002,
            "marketCap": {exact},
            "currency": "USD"
          }}]
        }}]
      }}
    }}"#
    );
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::default();
    let base = Url::parse(&format!(
        "{}/v1/finance/screener/predefined/saved",
        server.base_url()
    ))
    .unwrap();
    let response = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base)
        .cache_mode(CacheMode::Bypass)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    let market_cap = response.results[0]
        .market_cap
        .as_ref()
        .expect("market cap should map");
    assert_eq!(market_cap.amount(), paft::Decimal::from(exact));
    assert_eq!(
        response.results[0]
            .price
            .as_ref()
            .expect("price should map")
            .amount(),
        "1234567890.123456789012345678".parse::<Decimal>().unwrap()
    );
    assert_eq!(
        response.results[0].regular_market_change_percent,
        Some(Decimal::new(31_600_002, 9))
    );
}

#[tokio::test]
async fn predefined_screener_regular_market_volume_accepts_numeric_string() {
    let server = MockServer::start();
    let body = r#"{
      "finance": {
        "error": null,
        "result": [{
          "count": 1,
          "quotes": [{
            "symbol": "AAPL",
            "quoteType": "EQUITY",
            "regularMarketPrice": 190.0,
            "regularMarketVolume": "12345",
            "currency": "USD"
          }]
        }]
      }
    }"#;
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::default();
    let base = Url::parse(&format!(
        "{}/v1/finance/screener/predefined/saved",
        server.base_url()
    ))
    .unwrap();
    let response = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base)
        .cache_mode(CacheMode::Bypass)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    assert_eq!(response.count, Some(1));
    assert_eq!(response.results[0].regular_market_volume, Some(12_345));
}

#[tokio::test]
async fn predefined_screener_with_diagnostics_reports_invalid_currency_for_present_price() {
    let server = MockServer::start();
    let body = r#"{
      "finance": {
        "error": null,
        "result": [{
          "count": 1,
          "quotes": [{
            "symbol": "BADCUR",
            "quoteType": "EQUITY",
            "shortName": "Bad Currency Inc.",
            "regularMarketPrice": 10.0,
            "marketCap": 1000000,
            "currency": "!!!"
          }]
        }]
      }
    }"#;
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::default();
    let base = Url::parse(&format!(
        "{}/v1/finance/screener/predefined/saved",
        server.base_url()
    ))
    .unwrap();
    let response = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base)
        .cache_mode(CacheMode::Bypass)
        .fetch_with_diagnostics()
        .await
        .unwrap();

    mock.assert();
    assert!(response.data.results[0].price.is_none());
    assert!(response.data.results[0].market_cap.is_none());
    assert!(response.diagnostics.warnings.iter().any(|warning| {
        matches!(
            warning,
            YfWarning::OmittedPresentField {
                endpoint: "screener",
                path: "regularMarketPrice",
                key: Some(key),
                reason: ProjectionIssue::InvalidCurrency { code },
            } if key == "BADCUR" && code == "!!!"
        )
    }));
}

#[tokio::test]
async fn screener_yahoo_exchange_codes_normalize_without_diagnostics() {
    let server = MockServer::start();
    let body = r#"{
      "finance": {
        "error": null,
        "result": [{
          "count": 1,
          "quotes": [{
            "symbol": "SMH",
            "quoteType": "ETF",
            "shortName": "Semiconductor ETF",
            "exchange": "NGM",
            "currency": "USD",
            "regularMarketPrice": 10.0
          }]
        }]
      }
    }"#;
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::default();
    let base = Url::parse(&format!(
        "{}/v1/finance/screener/predefined/saved",
        server.base_url()
    ))
    .unwrap();
    let response = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base.clone())
        .cache_mode(CacheMode::Bypass)
        .fetch_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(
        response.data.results[0]
            .exchange
            .as_ref()
            .map(std::string::ToString::to_string)
            .as_deref(),
        Some("NASDAQ")
    );
    assert_eq!(
        response.data.results[0].yahoo_exchange,
        Some(YahooExchangeCode::Ngm)
    );
    assert!(response.diagnostics.warnings.is_empty());

    ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base)
        .cache_mode(CacheMode::Bypass)
        .strict()
        .fetch()
        .await
        .unwrap();

    mock.assert_calls(2);
}

#[tokio::test]
async fn malformed_screener_optional_fields_are_omitted_without_losing_result() {
    let server = MockServer::start();
    let body = r#"{
      "finance": {
        "error": null,
        "result": [{
          "count": "not-a-count",
          "quotes": [
            {
              "symbol": "BADQUOTE",
              "quoteType": "EQUITY",
              "regularMarketPrice": "not-a-number"
            },
            {
              "symbol": "AAPL",
              "quoteType": "EQUITY",
              "regularMarketPrice": 190.0,
              "currency": "USD"
            }
          ]
        }]
      }
    }"#;
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::default();
    let base = Url::parse(&format!(
        "{}/v1/finance/screener/predefined/saved",
        server.base_url()
    ))
    .unwrap();
    let response = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base.clone())
        .cache_mode(CacheMode::Bypass)
        .fetch_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.results.len(), 2);
    assert_eq!(response.data.count, None);
    assert_eq!(response.data.results[0].symbol.as_deref(), Some("BADQUOTE"));
    assert!(response.data.results[0].price.is_none());
    assert_eq!(response.data.results[1].symbol.as_deref(), Some("AAPL"));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            endpoint: "screener",
            path: "regularMarketPrice",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "regularMarketPrice",
                ..
            },
        } if key == "BADQUOTE"
    )));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            endpoint: "screener",
            path: "count",
            key: None,
            reason: ProjectionIssue::InvalidField { field: "count", .. },
        }
    )));

    let err = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base)
        .cache_mode(CacheMode::Bypass)
        .strict()
        .fetch()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn predefined_screener_401_with_stale_cached_crumb_refreshes_before_retry() {
    let server = MockServer::start();
    let bare_request = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US")
            .is_true(|req| !req.query_params().iter().any(|(k, _)| k == "crumb"));
        then.status(401);
    });

    let (cookie, crumb) = crate::common::mock_cookie_crumb(&server);

    let stale = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US")
            .query_param("crumb", "stale-crumb");
        then.status(401);
    });

    let ok = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/screener/predefined/saved")
            .query_param("scrIds", "day_gainers")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US")
            .query_param("crumb", "crumb-value");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("screener_predefined", "day_gainers"));
    });

    let client = YfClient::builder()
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        ._preauth("cookie", "stale-crumb")
        .build()
        .unwrap();
    let base = Url::parse(&format!(
        "{}/v1/finance/screener/predefined/saved",
        server.base_url()
    ))
    .unwrap();
    let response = ScreenerBuilder::predefined(&client, PredefinedScreener::DayGainers)
        .predefined_screener_base(base)
        .cache_mode(CacheMode::Bypass)
        .fetch()
        .await
        .unwrap();

    assert_eq!(
        bare_request.calls(),
        0,
        "cached OptionalCrumb credentials should be tried before a bare request"
    );
    stale.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();
    assert!(!response.results.is_empty());
}

#[tokio::test]
async fn offline_custom_equity_query_posts_python_wire_shape() {
    let server = MockServer::start();
    let expected_body = json!({
        "offset": 0,
        "count": 25,
        "sortField": "ticker",
        "sortType": "DESC",
        "userId": "",
        "userIdType": "guid",
        "quoteType": "EQUITY",
        "query": {
            "operator": "AND",
            "operands": [
                {"operator": "GT", "operands": ["percentchange", 3.0]},
                {"operator": "EQ", "operands": ["region", "us"]}
            ]
        }
    });

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/finance/screener")
            .header("content-type", "application/json")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US")
            .json_body(expected_body);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("screener_custom", "equity"));
    });

    let client = YfClient::default();
    let query = EquityQuery::and(vec![
        equity_fields::PERCENT_CHANGE.gt(yfinance_rs::PercentPoints::new(
            yfinance_rs::Decimal::new(30, 1),
        )),
        equity_fields::REGION.eq(Region::Us),
    ])
    .unwrap();
    let base = Url::parse(&format!("{}/v1/finance/screener", server.base_url())).unwrap();
    let response = ScreenerBuilder::equity(&client, query)
        .screener_base(base)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    assert!(!response.results.is_empty());
    assert!(response.results[0].symbol.is_some());
}

#[tokio::test]
async fn explicit_custom_screener_cache_mode_uses_post_body_cache() {
    let server = MockServer::start();
    let expected_body = json!({
        "offset": 0,
        "count": 25,
        "sortField": "ticker",
        "sortType": "DESC",
        "userId": "",
        "userIdType": "guid",
        "quoteType": "EQUITY",
        "query": {
            "operator": "AND",
            "operands": [
                {"operator": "GT", "operands": ["percentchange", 3.0]},
                {"operator": "EQ", "operands": ["region", "us"]}
            ]
        }
    });

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/finance/screener")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US")
            .json_body(expected_body);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("screener_custom", "equity"));
    });

    let client = YfClient::builder()
        .cache_ttl(Duration::from_mins(1))
        .build()
        .unwrap();
    let base = Url::parse(&format!("{}/v1/finance/screener", server.base_url())).unwrap();

    for _ in 0..2 {
        let query = EquityQuery::and(vec![
            equity_fields::PERCENT_CHANGE.gt(yfinance_rs::PercentPoints::new(
                yfinance_rs::Decimal::new(30, 1),
            )),
            equity_fields::REGION.eq(Region::Us),
        ])
        .unwrap();
        let response = ScreenerBuilder::equity(&client, query)
            .screener_base(base.clone())
            .cache_mode(CacheMode::Use)
            .fetch()
            .await
            .unwrap();
        assert!(!response.results.is_empty());
    }

    mock.assert_calls(1);
}

#[tokio::test]
async fn custom_screener_403_with_stale_cached_crumb_refreshes_before_retry() {
    let server = MockServer::start();
    let expected_body = json!({
        "offset": 0,
        "count": 25,
        "sortField": "ticker",
        "sortType": "DESC",
        "userId": "",
        "userIdType": "guid",
        "quoteType": "EQUITY",
        "query": {
            "operator": "AND",
            "operands": [
                {"operator": "GT", "operands": ["percentchange", 3.0]},
                {"operator": "EQ", "operands": ["region", "us"]}
            ]
        }
    });

    let bare_request = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/finance/screener")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US")
            .is_true(|req| !req.query_params().iter().any(|(k, _)| k == "crumb"))
            .json_body(expected_body.clone());
        then.status(403);
    });

    let (cookie, crumb) = crate::common::mock_cookie_crumb(&server);

    let stale = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/finance/screener")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US")
            .query_param("crumb", "stale-crumb")
            .json_body(expected_body.clone());
        then.status(403);
    });

    let ok = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/finance/screener")
            .query_param("corsDomain", "finance.yahoo.com")
            .query_param("formatted", "false")
            .query_param("lang", "en-US")
            .query_param("region", "US")
            .query_param("crumb", "crumb-value")
            .json_body(expected_body.clone());
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("screener_custom", "equity"));
    });

    let client = YfClient::builder()
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        ._preauth("cookie", "stale-crumb")
        .build()
        .unwrap();
    let query = EquityQuery::and(vec![
        equity_fields::PERCENT_CHANGE.gt(yfinance_rs::PercentPoints::new(
            yfinance_rs::Decimal::new(30, 1),
        )),
        equity_fields::REGION.eq(Region::Us),
    ])
    .unwrap();
    let base = Url::parse(&format!("{}/v1/finance/screener", server.base_url())).unwrap();
    let response = ScreenerBuilder::equity(&client, query)
        .screener_base(base)
        .cache_mode(CacheMode::Bypass)
        .fetch()
        .await
        .unwrap();

    assert_eq!(
        bare_request.calls(),
        0,
        "cached OptionalCrumb credentials should be tried before a bare request"
    );
    stale.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();
    assert!(!response.results.is_empty());
}
