#[tokio::test]
#[ignore]
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
        assert!(res.series["AAPL"].len() > 0);
        assert!(res.series["MSFT"].len() > 0);
    }
}

#[tokio::test]
#[ignore]
async fn live_download_for_record() {
    // This test is only meant to *record* fixtures.
    if !crate::common::is_recording() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    // This will hit Yahoo live and record:
    //   tests/fixtures/history_chart_AAPL.json
    //   tests/fixtures/history_chart_MSFT.json
    // (same filenames the offline tests replay)
    let _ = yfinance_rs::DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .run()
        .await;
}
