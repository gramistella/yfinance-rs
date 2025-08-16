use tokio::time::{timeout, Duration};
use url::Url;

#[tokio::test]
async fn stream_offline_uses_recorded_fixture() {
    // Serve the cached MULTI-quote fixture via a mock, so this test never hits the network.
    let server = crate::common::setup_server();

    // This fixture is written during a YF_RECORD live sweep by Stream (endpoint=quote_v7, symbol=MULTI).
    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL");
        then.status(200)
            .header("content-type","application/json")
            .body(crate::common::fixture("quote_v7", "MULTI", "json"));
    });

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    let builder = yfinance_rs::StreamBuilder::new(&client)
        .unwrap()
        .symbols(["AAPL"])
        .quote_base(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .interval(Duration::from_millis(40))
        .diff_only(false);

    let (handle, mut rx) = builder.start().unwrap();

    let got = timeout(Duration::from_secs(3), rx.recv()).await;
    handle.abort();

    mock.assert();

    let update = got.expect("timed out waiting for cached stream update")
                    .expect("stream closed without emitting an update");

    assert_eq!(update.symbol, "AAPL");
    assert!(update.last_price.unwrap_or(0.0) > 0.0, "cached price should be > 0");
}
