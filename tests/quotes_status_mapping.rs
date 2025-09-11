use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;

#[tokio::test]
async fn quotes_v7_404_maps_to_not_found() {
    let server = MockServer::start();

    // Prepare a v7 endpoint that returns 404
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "MISSING");
        then.status(404)
            .header("content-type", "application/json")
            .body("{}");
    });

    let client = yfinance_rs::YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .retry_enabled(false)
        .build()
        .unwrap();

    let symbols = ["MISSING".to_string()];
    let err = yfinance_rs::quote::quotes(&client, symbols.iter().cloned())
        .await
        .unwrap_err();

    mock.assert();

    match err {
        yfinance_rs::YfError::NotFound { url } => {
            assert!(url.contains("/v7/finance/quote"));
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn quotes_v7_429_maps_to_rate_limited() {
    let server = MockServer::start();

    // Prepare a v7 endpoint that returns 429
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL");
        then.status(429)
            .header("content-type", "application/json")
            .body("{}");
    });

    let client = yfinance_rs::YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .retry_enabled(false)
        .build()
        .unwrap();

    let symbols = ["AAPL".to_string()];
    let err = yfinance_rs::quote::quotes(&client, symbols.iter().cloned())
        .await
        .unwrap_err();

    mock.assert();

    match err {
        yfinance_rs::YfError::RateLimited { url } => {
            assert!(url.contains("/v7/finance/quote"));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}
