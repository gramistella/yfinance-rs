use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
#[ignore]
async fn live_isin_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = YfClient::builder().build().unwrap();

    // Test a company. This will trigger a Yahoo quote request and a Business Insider request.
    // Both responses will be recorded as fixtures if YF_RECORD=1.
    let ticker_company = Ticker::new(client.clone(), "AAPL");
    let isin_company = ticker_company.isin().await.unwrap();

    // Test a fund.
    let ticker_fund = Ticker::new(client.clone(), "QQQ");
    let isin_fund = ticker_fund.isin().await.unwrap();

    if !crate::common::is_recording() {
        assert_eq!(
            isin_company.as_deref(),
            Some("US0378331005"),
            "Expected correct ISIN for AAPL"
        );
        assert_eq!(
            isin_fund.as_deref(),
            Some("US46090E1038"),
            "Expected correct ISIN for QQQ"
        );
    }
}