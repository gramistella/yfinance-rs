use paft::domain::IdentifierScheme;
use tokio::time::{Duration, timeout};
use url::Url;
use yfinance_rs::StreamMethod;
use yfinance_rs::core::client::CacheMode;

#[tokio::test]
async fn stream_websocket_fallback_to_polling_offline() {
    let server = crate::common::setup_server();

    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("quote_v7", "AAPL", "json"));
    });

    let client = yfinance_rs::YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_stream(Url::parse("wss://invalid-url-for-testing.invalid/").unwrap())
        .build()
        .unwrap();

    let builder = yfinance_rs::StreamBuilder::new(&client)
        .symbols(["AAPL"])
        .method(StreamMethod::WebsocketWithFallback)
        .interval(Duration::from_millis(40));

    let (handle, mut rx) = builder.start().unwrap();

    let got = timeout(Duration::from_secs(3), rx.recv()).await;
    handle.abort();

    mock.assert();

    let update = got
        .expect("timed out waiting for cached stream update")
        .expect("stream closed without emitting an update");

    match update.instrument.id() {
        IdentifierScheme::Security(s) => assert_eq!(s.symbol.as_str(), "AAPL"),
        IdentifierScheme::Prediction(_) => panic!("unexpected instrument identifier scheme"),
    }
    assert!(
        update
            .price
            .as_ref()
            .map_or(0.0, yfinance_rs::core::conversions::money_to_f64)
            > 0.0,
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
            .body(crate::common::fixture("quote_v7", "MSFT", "json"));
    });

    let client = yfinance_rs::YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();

    let builder = yfinance_rs::StreamBuilder::new(&client)
        .symbols(["MSFT"])
        .method(StreamMethod::Polling)
        .interval(Duration::from_millis(50));

    let (handle, mut rx) = builder.start().unwrap();
    let got = timeout(Duration::from_secs(3), rx.recv()).await;
    handle.abort();
    mock.assert();

    assert!(got.is_ok());
}

#[tokio::test]
async fn stream_polling_emits_on_volume_only_change_with_diff_only() {
    let server = crate::common::setup_server();

    // First response: price P, volume V1
    let body1 = r#"{
        "quoteResponse": {
            "result": [
                {
                    "symbol": "MSFT",
                    "regularMarketPrice": 420.00,
                    "regularMarketPreviousClose": 420.00,
                    "regularMarketVolume": 1000,
                    "currency": "USD"
                }
            ],
            "error": null
        }
    }"#;

    // Second response: same price P, higher volume V2
    let body2 = r#"{
        "quoteResponse": {
            "result": [
                {
                    "symbol": "MSFT",
                    "regularMarketPrice": 420.00,
                    "regularMarketPreviousClose": 420.00,
                    "regularMarketVolume": 1500,
                    "currency": "USD"
                }
            ],
            "error": null
        }
    }"#;

    // Set up two sequential mocks. The first is limited to a single call so the second one is used next.
    let mut m1 = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "MSFT");
        then.status(200)
            .header("content-type", "application/json")
            .body(body1);
    });

    let client = yfinance_rs::YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();

    // diff_only defaults to true; ensure we bypass cache so each poll hits the server
    let builder = yfinance_rs::StreamBuilder::new(&client)
        .symbols(["MSFT"])
        .method(StreamMethod::Polling)
        .interval(Duration::from_millis(100))
        .cache_mode(CacheMode::Bypass);

    let (handle, mut rx) = builder.start().unwrap();

    // First tick (price change from None -> P) should emit
    let first = timeout(Duration::from_secs(3), rx.recv()).await;
    // After first emission, switch the mock to return a higher volume
    m1.delete();
    let _m2 = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "MSFT");
        then.status(200)
            .header("content-type", "application/json")
            .body(body2);
    });

    // Second tick: price unchanged, volume increased -> must emit
    let second = timeout(Duration::from_secs(3), rx.recv()).await;

    handle.abort();

    let first = first
        .expect("timed out waiting for first update")
        .expect("stream closed before first update");
    let second = second
        .expect("timed out waiting for second update")
        .expect("stream closed before second update");

    // Price should be unchanged between ticks in this scenario
    let first_price = first
        .price
        .as_ref()
        .map_or(f64::NAN, yfinance_rs::core::conversions::money_to_f64);
    let second_price = second
        .price
        .as_ref()
        .map_or(f64::NAN, yfinance_rs::core::conversions::money_to_f64);
    assert!(
        (first_price - second_price).abs() < 1e-9,
        "price should be unchanged when only volume increases"
    );

    assert!(
        second.volume.unwrap_or(0) > 0,
        "second update should carry positive volume delta when price is unchanged"
    );
}
