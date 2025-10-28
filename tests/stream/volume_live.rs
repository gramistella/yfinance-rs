use tokio::time::{Duration, timeout};

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API; also records fixtures if YF_RECORD=1"]
async fn live_stream_volume_delta_presence() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let builder = yfinance_rs::StreamBuilder::new(&client)
        .symbols(["BTC-USD"]) // active 24/7 symbol for reliable ticks
        .method(yfinance_rs::StreamMethod::Websocket);

    let (handle, mut rx) = builder.start().unwrap();

    // First tick may contain whole day_volume; we need two ticks to see delta presence
    let first = timeout(Duration::from_secs(90), rx.recv()).await;
    let second = timeout(Duration::from_secs(90), rx.recv()).await;

    handle.abort();

    let _first = first
        .expect("no first live update within timeout")
        .expect("stream closed without first update");
    let second = second
        .expect("no second live update within timeout")
        .expect("stream closed before second update");

    // After the first tick, per-update volume should be populated
    assert!(
        second.volume.is_some(),
        "second update should carry per-tick volume"
    );
}
