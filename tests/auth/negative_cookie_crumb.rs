use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{YfClient, YfError};

#[tokio::test]
async fn missing_set_cookie_header_is_an_error() {
    let server = MockServer::start();
    let sym = "AAPL";

    // Cookie endpoint returns 200 but no Set-Cookie header.
    let cookie = server.mock(|when, then| {
        when.method(GET).path("/consent");
        then.status(200); // no set-cookie
    });
    let crumb = server.mock(|when, then| {
        // won't be reached, but good to have
        when.method(GET).path("/v1/test/getcrumb");
        then.status(200).body("crumb-value");
    });

    // Any API body (won't be reached if ensure_credentials fails early)
    let api = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":[],"error":null}}"#);
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        .build()
        .unwrap();

    let err = yfinance_rs::profile::load_profile(&client, sym)
        .await
        .unwrap_err();
    cookie.assert();

    match err {
        YfError::Auth(s) => assert!(s.contains("No cookie received"), "unexpected error: {s}"),
        other => panic!("expected Auth error, got {other:?}"),
    }
    assert_eq!(
        crumb.hits(),
        0,
        "crumb endpoint should not be called if cookie fails"
    );
    assert_eq!(
        api.hits(),
        0,
        "API should not be called if credentials fail"
    );
}

#[tokio::test]
async fn invalid_crumb_body_is_an_error() {
    let server = MockServer::start();
    let sym = "AAPL";

    // Proper Set-Cookie
    let _cookie = server.mock(|when, then| {
        when.method(GET).path("/consent");
        then.status(200).header("set-cookie", "A=B; Path=/");
    });
    // Crumb endpoint returns "{}" which should be rejected
    let _crumb = server.mock(|when, then| {
        when.method(GET).path("/v1/test/getcrumb");
        then.status(200).body("{}");
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        .build()
        .unwrap();

    let err = yfinance_rs::profile::load_profile(&client, sym)
        .await
        .unwrap_err();

    match err {
        YfError::Auth(s) => assert!(s.contains("Received invalid crumb"), "unexpected: {s}"),
        other => panic!("expected Auth error, got {other:?}"),
    }
}
