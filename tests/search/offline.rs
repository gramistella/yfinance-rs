use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ProjectionIssue, SearchBuilder, YfClient, YfError, YfWarning};

fn fixture(endpoint: &str, key: &str) -> String {
    crate::common::fixture(endpoint, key, "json")
}

#[tokio::test]
async fn offline_search_uses_recorded_fixture() {
    // Query we'll use for fixture key
    let query = "apple";
    let server = MockServer::start();

    // Mock Yahoo /v1/finance/search with expected params
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10")
            .is_true(|req| {
                !req.query_params()
                    .iter()
                    .any(|(key, _)| key == "newsCount" || key == "listsCount")
            });
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("search_v1", query));
    });

    let client = YfClient::builder().build().unwrap();

    let resp = SearchBuilder::new(&client, query)
        .search_base(Url::parse(&format!("{}/v1/finance/search", server.base_url())).unwrap())
        .fetch()
        .await
        .unwrap();

    mock.assert();
    // At least one result expected (record with YF_RECORD=1 first)
    assert!(!resp.results.is_empty(), "record with YF_RECORD=1 first");
    assert!(
        resp.results
            .iter()
            .any(|q| q.instrument.symbol.as_str() == "AAPL")
    );
}

#[tokio::test]
async fn search_rejects_empty_query_before_request() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/v1/finance/search");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quotes":[]}"#);
    });

    let client = YfClient::builder().build().unwrap();
    let base = Url::parse(&format!("{}/v1/finance/search", server.base_url())).unwrap();

    for query in ["", " \t\n "] {
        let err = SearchBuilder::new(&client, query)
            .search_base(base.clone())
            .fetch()
            .await
            .unwrap_err();

        match err {
            YfError::InvalidParams(message) => assert!(message.contains("query")),
            other => panic!("expected InvalidParams, got {other:?}"),
        }
    }

    mock.assert_calls(0);
}

#[tokio::test]
async fn search_403_with_stale_cached_crumb_refreshes_before_retry() {
    let query = "apple";
    let server = MockServer::start();

    let bare = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10")
            .is_true(|req| !req.query_params().iter().any(|(k, _)| k == "crumb"));
        then.status(403);
    });

    let (cookie, crumb) = crate::common::mock_cookie_crumb(&server);

    let stale = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10")
            .query_param("crumb", "stale-crumb");
        then.status(403);
    });

    let ok = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10")
            .query_param("crumb", "crumb-value");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("search_v1", query));
    });

    let client = YfClient::builder()
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        ._preauth("cookie", "stale-crumb")
        .build()
        .unwrap();

    let resp = SearchBuilder::new(&client, query)
        .search_base(Url::parse(&format!("{}/v1/finance/search", server.base_url())).unwrap())
        .fetch()
        .await
        .unwrap();

    assert_eq!(
        bare.calls(),
        0,
        "cached OptionalCrumb credentials should be tried before a bare request"
    );
    stale.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();
    assert!(
        resp.results
            .iter()
            .any(|q| q.instrument.symbol.as_str() == "AAPL")
    );
}

#[tokio::test]
async fn invalid_search_exchange_is_reported_as_projection_loss() {
    let query = "bad exchange";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quotes": [{
                    "symbol": "BADX",
                    "quoteType": "EQUITY",
                    "exchange": "!!!"
                  }]
                }"#,
            );
    });

    let client = YfClient::builder().build().unwrap();
    let base = Url::parse(&format!("{}/v1/finance/search", server.base_url())).unwrap();

    let response = SearchBuilder::new(&client, query)
        .search_base(base.clone())
        .fetch_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.results.len(), 1);
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "quotes[].exchange",
            reason: ProjectionIssue::InvalidField {
                field: "exchange",
                ..
            },
            ..
        }
    )));

    let err = SearchBuilder::new(&client, query)
        .search_base(base)
        .strict()
        .fetch()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn yahoo_search_exchange_codes_normalize_without_diagnostics() {
    let query = "nasdaq code";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quotes": [{
                    "symbol": "AAPL",
                    "quoteType": "EQUITY",
                    "exchange": "NMS"
                  }]
                }"#,
            );
    });

    let client = YfClient::builder().build().unwrap();
    let base = Url::parse(&format!("{}/v1/finance/search", server.base_url())).unwrap();

    let response = SearchBuilder::new(&client, query)
        .search_base(base.clone())
        .fetch_with_diagnostics()
        .await
        .unwrap();

    mock.assert();
    assert_eq!(
        response.data.results[0]
            .instrument
            .exchange
            .as_ref()
            .map(std::string::ToString::to_string)
            .as_deref(),
        Some("NASDAQ")
    );
    assert!(response.diagnostics.warnings.is_empty());

    SearchBuilder::new(&client, query)
        .search_base(base)
        .strict()
        .fetch()
        .await
        .unwrap();
}

#[tokio::test]
async fn malformed_optional_search_name_is_omitted_without_losing_result() {
    let query = "malformed optional";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "count": {"unexpected": true},
                  "quotes": [{
                    "symbol": "AAPL",
                    "quoteType": "EQUITY",
                    "exchange": "NasdaqGS",
                    "longname": 42,
                    "shortname": "Apple Inc."
                  }]
                }"#,
            );
    });

    let client = YfClient::builder().build().unwrap();
    let base = Url::parse(&format!("{}/v1/finance/search", server.base_url())).unwrap();

    let response = SearchBuilder::new(&client, query)
        .search_base(base.clone())
        .fetch_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.results.len(), 1);
    assert_eq!(response.data.results[0].instrument.symbol.as_str(), "AAPL");
    assert_eq!(response.data.results[0].name.as_deref(), Some("Apple Inc."));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            endpoint: "search",
            path: "quotes[].longname",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "quotes[].longname",
                ..
            },
        } if key == "AAPL"
    )));

    let err = SearchBuilder::new(&client, query)
        .search_base(base)
        .strict()
        .fetch()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn malformed_search_quote_is_dropped_without_losing_valid_siblings() {
    let query = "malformed row";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quotes": [
                    { "symbol": 123, "quoteType": "EQUITY" },
                    { "symbol": "AAPL", "quoteType": "EQUITY", "exchange": "NasdaqGS" }
                  ]
                }"#,
            );
    });

    let client = YfClient::builder().build().unwrap();
    let base = Url::parse(&format!("{}/v1/finance/search", server.base_url())).unwrap();

    let response = SearchBuilder::new(&client, query)
        .search_base(base.clone())
        .fetch_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.results.len(), 1);
    assert_eq!(response.data.results[0].instrument.symbol.as_str(), "AAPL");
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::DroppedItem {
            endpoint: "search",
            item: "search_result",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "symbol",
                ..
            },
        } if key == "quotes[0]"
    )));

    let err = SearchBuilder::new(&client, query)
        .search_base(base)
        .strict()
        .fetch()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}
