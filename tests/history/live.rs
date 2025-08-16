#[tokio::test]
#[ignore]
async fn live_history_smoke() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let bars = yfinance_rs::HistoryBuilder::new(&client, "AAPL")
        .fetch()
        .await
        .unwrap();

    if !crate::common::is_recording() {
        assert!(!bars.is_empty());
        assert!(bars[0].open > 0.0 && bars[0].close > 0.0);
    }
}

#[tokio::test]
#[ignore]
async fn live_history_for_record() {
    if !crate::common::is_recording() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let _ = yfinance_rs::HistoryBuilder::new(&client, "AAPL")
        .fetch()
        .await;
    let _ = yfinance_rs::HistoryBuilder::new(&client, "MSFT")
        .fetch()
        .await;
}
