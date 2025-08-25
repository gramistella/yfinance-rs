#[tokio::test]
#[ignore]
async fn live_analysis_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    // Trend
    {
        let t = yfinance_rs::Ticker::new(client.clone(), "AAPL").unwrap();
        let _ = t.recommendations().await.unwrap();
    }
    // Summary
    {
        let t = yfinance_rs::Ticker::new(client.clone(), "MSFT").unwrap();
        let _ = t.recommendations_summary().await.unwrap();
    }
    // Upgrades/Downgrades
    {
        let mut t = yfinance_rs::Ticker::new(client.clone(), "GOOGL").unwrap();
        let _ = t.upgrades_downgrades().await.unwrap();
    }

    // If not recording, at least assert the calls returned something sane.
    if !crate::common::is_recording() {
        let t = yfinance_rs::Ticker::new(client, "AAPL").unwrap();
        let rows = t.recommendations().await.unwrap();
        // Smoke check: when hitting live, expect at least one row
        assert!(!rows.is_empty());
    }
}
