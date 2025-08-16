/// Live smoke test (ignored by default). Run in record mode to capture fixtures:
///   YF_RECORD=1 cargo test --features test-mode -- --include-ignored --test-threads=1
#[tokio::test]
#[ignore]
async fn live_stream_smoke() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    let builder = yfinance_rs::StreamBuilder::new(&client)
        .unwrap()
        .symbols(["AAPL"])
        .interval(std::time::Duration::from_millis(400))
        .diff_only(false);

    let (handle, mut rx) = builder.start().unwrap();

    use tokio::time::{timeout, Duration};
    let got = timeout(Duration::from_secs(8), rx.recv()).await;

    handle.abort();

    let update = got.expect("no live update within timeout")
                    .expect("stream closed without emitting");
    assert_eq!(update.symbol, "AAPL");
    assert!(update.last_price.unwrap_or(0.0) > 0.0);
}
