use httpmock::{Method::GET, MockServer};
use serde_json::Value;
use url::Url;
use yfinance_rs::{Ticker, YfClient, YfError};

fn preauthed_client(server: &MockServer) -> YfClient {
    YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap()
}

#[tokio::test]
async fn offline_index_components_match_fixture() {
    let sym = "^FCHI";
    let server = MockServer::start();
    let body = crate::common::fixture("components_api_components", sym, "json");

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "components")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
    });

    let components = Ticker::new(&preauthed_client(&server), sym)
        .components()
        .await
        .unwrap();

    mock.assert();

    let v: Value = serde_json::from_str(&body).unwrap();
    let expected: Vec<String> = v["quoteSummary"]["result"][0]["components"]["components"]
        .as_array()
        .expect("components array in fixture")
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();

    assert!(
        !expected.is_empty(),
        "fixture should list index members. Did you run `just test-record ticker components`?"
    );
    assert_eq!(components, expected);
}

#[tokio::test]
async fn offline_not_found_returns_empty_components() {
    let sym = "AAPL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "components");
        then.status(404)
            .header("content-type", "application/json")
            .body(
                r#"{"quoteSummary":{"result":null,"error":{"code":"Not Found","description":"No fundamentals data found for symbol: AAPL"}}}"#,
            );
    });

    let components = Ticker::new(&preauthed_client(&server), sym)
        .components()
        .await
        .unwrap();

    mock.assert();
    assert!(components.is_empty());
}

#[tokio::test]
async fn offline_missing_components_module_returns_empty() {
    let sym = "^GSPC";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "components");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":[{}],"error":null}}"#);
    });

    let components = Ticker::new(&preauthed_client(&server), sym)
        .components()
        .await
        .unwrap();

    mock.assert();
    assert!(components.is_empty());
}

#[tokio::test]
async fn offline_server_error_is_still_an_error() {
    let sym = "^FCHI";
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "components");
        then.status(500).body("oops");
    });

    let err = Ticker::new(&preauthed_client(&server), sym)
        .retry_policy(None)
        .components()
        .await
        .unwrap_err();

    assert!(
        !matches!(err, YfError::NotFound { .. }),
        "unexpected error: {err:?}"
    );
}
