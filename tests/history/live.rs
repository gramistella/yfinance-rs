#[tokio::test]
#[ignore]
async fn live_history_smoke() {
    if std::env::var("YF_LIVE").ok().as_deref() != Some("1")
        && std::env::var("YF_RECORD").ok().as_deref() != Some("1")
    { return; }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let bars = yfinance_rs::HistoryBuilder::new(&client, "AAPL").fetch().await.unwrap();

    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        assert!(!bars.is_empty());
        assert!(bars[0].open > 0.0 && bars[0].close > 0.0);
    }
}

#[tokio::test]
#[ignore]
async fn live_history_for_record() {
    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") { return; }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let _ = yfinance_rs::HistoryBuilder::new(&client, "AAPL").fetch().await;
    let _ = yfinance_rs::HistoryBuilder::new(&client, "MSFT").fetch().await;
}
