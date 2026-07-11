use yfinance_rs::core::conversions::*;

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
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
        assert!(money_to_f64(&bars[0].ohlc.open) > 0.0 && money_to_f64(&bars[0].ohlc.close) > 0.0);
    }
}

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
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

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_intraday_history_for_record() {
    if !crate::common::is_recording() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let bars = yfinance_rs::HistoryBuilder::new(&client, "IBM")
        .range(yfinance_rs::Range::D5)
        .interval(yfinance_rs::Interval::I5m)
        .fetch()
        .await
        .unwrap();
    assert!(bars.len() > 100);
}
