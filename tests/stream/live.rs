use yfinance_rs::StreamMethod;

#[tokio::test]
#[ignore]
async fn live_stream_smoke() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    let builder = yfinance_rs::StreamBuilder::new(&client)
        .unwrap()
        .symbols(["BTC-USD"]) // Switched to a 24/7 symbol
        .method(StreamMethod::Websocket);

    let (handle, mut rx) = builder.start().unwrap();

    use tokio::time::{Duration, timeout};
    let got = timeout(Duration::from_secs(90), rx.recv()).await;

    handle.abort();

    let update = got
        .expect("no live update within timeout")
        .expect("stream closed without emitting");

    // Updated assertion for the new symbol
    assert_eq!(update.symbol, "BTC-USD");
    assert!(update.last_price.unwrap_or(0.0) > 0.0);
}
