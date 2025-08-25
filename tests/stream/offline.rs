use tokio::time::{Duration, timeout};
use url::Url;
use yfinance_rs::StreamMethod;

#[tokio::test]
async fn stream_websocket_fallback_to_polling_offline() {
    let server = crate::common::setup_server();

    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("quote_v7", "MULTI", "json"));
    });

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    let builder = yfinance_rs::StreamBuilder::new(&client)
        .unwrap()
        .symbols(["AAPL"])
        .quote_base(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        // Provide an invalid websocket URL to force fallback
        .stream_url(Url::parse("wss://invalid-url-for-testing.invalid/").unwrap())
        .method(StreamMethod::WebsocketWithFallback)
        .interval(Duration::from_millis(40));

    let (handle, mut rx) = builder.start().unwrap();

    let got = timeout(Duration::from_secs(3), rx.recv()).await;
    handle.abort();

    // The polling mock should have been hit
    mock.assert();

    let update = got
        .expect("timed out waiting for cached stream update")
        .expect("stream closed without emitting an update");

    assert_eq!(update.symbol, "AAPL");
    assert!(
        update.last_price.unwrap_or(0.0) > 0.0,
        "cached price should be > 0"
    );
}

#[tokio::test]
async fn stream_polling_explicitly_offline() {
    let server = crate::common::setup_server();

    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "MSFT");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("quote_v7", "MULTI", "json"));
    });

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    let builder = yfinance_rs::StreamBuilder::new(&client)
        .unwrap()
        .symbols(["MSFT"])
        .quote_base(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .method(StreamMethod::Polling)
        .interval(Duration::from_millis(50));

    let (handle, mut rx) = builder.start().unwrap();
    let got = timeout(Duration::from_secs(3), rx.recv()).await;
    handle.abort();
    mock.assert();

    assert!(got.is_ok());
}
