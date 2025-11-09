use paft::domain::IdentifierScheme;

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_download_smoke() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let res = yfinance_rs::DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .run()
        .await
        .unwrap();

    if !crate::common::is_recording() {
        let aapl = res
            .entries
            .iter()
            .find(|e| match e.instrument.id() {
                IdentifierScheme::Security(s) => s.symbol.as_ref() == "AAPL",
                IdentifierScheme::Prediction(_) => false,
            })
            .unwrap();
        let msft = res
            .entries
            .iter()
            .find(|e| match e.instrument.id() {
                IdentifierScheme::Security(s) => s.symbol.as_ref() == "MSFT",
                IdentifierScheme::Prediction(_) => false,
            })
            .unwrap();
        assert!(!aapl.history.candles.is_empty());
        assert!(!msft.history.candles.is_empty());
    }
}

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_download_for_record() {
    // Only writes fixtures when YF_RECORD=1
    if !crate::common::is_recording() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    // This will hit Yahoo live and record:
    //   tests/fixtures/history_chart_AAPL.json
    //   tests/fixtures/history_chart_MSFT.json
    let _ = yfinance_rs::DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .run()
        .await;
}
