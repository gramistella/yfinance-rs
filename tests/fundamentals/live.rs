#[tokio::test]
#[ignore]
async fn live_fundamentals_smoke() {
    // Mirrors the live/record pattern used elsewhere
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    // Use distinct symbols to avoid fixture clobbering since endpoint name is the same
    // ("fundamentals_api_<SYMBOL>.json")
    // income (quarterly)
    {
        let t = yfinance_rs::Ticker::new(&client, "AAPL");
        let _ = t.quarterly_income_stmt().await.unwrap();
    }
    // balance (annual)
    {
        let t = yfinance_rs::Ticker::new(&client, "MSFT");
        let _ = t.balance_sheet().await.unwrap();
    }
    // cashflow (annual)
    {
        let t = yfinance_rs::Ticker::new(&client, "GOOGL");
        let _ = t.cashflow().await.unwrap();
    }
    // earnings
    {
        let t = yfinance_rs::Ticker::new(&client, "AMZN");
        let _ = t.earnings().await.unwrap();
    }
    // calendar
    {
        let t = yfinance_rs::Ticker::new(&client, "META");
        let _ = t.calendar().await.unwrap();
    }

    if !crate::common::is_recording() {
        // If not recording, at least assert we got *some* data from live
        // (No strict expectations; shapes vary by company)
        let t = yfinance_rs::Ticker::new(&client, "AAPL");
        let income = t.quarterly_income_stmt().await.unwrap();
        assert!(!income.is_empty());
    }

    if !crate::common::is_recording() {
        let t = yfinance_rs::Ticker::new(&client, "MSFT");
        let balance_sheet = t.balance_sheet().await.unwrap();
        assert!(!balance_sheet.is_empty());
        assert!(balance_sheet[0].shares_outstanding.is_some());
    }
}

#[tokio::test]
#[ignore]
async fn live_fundamentals_for_record() {
    // Only run when actually recording; this populates fixtures for offline tests.
    if !crate::common::is_recording() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    let t1 = yfinance_rs::Ticker::new(&client, "AAPL");
    let _ = t1.quarterly_income_stmt().await;

    let t2 = yfinance_rs::Ticker::new(&client, "MSFT");
    let _ = t2.balance_sheet().await;

    let t3 = yfinance_rs::Ticker::new(&client, "GOOGL");
    let _ = t3.cashflow().await;

    let t4 = yfinance_rs::Ticker::new(&client, "AMZN");
    let _ = t4.earnings().await;

    let t5 = yfinance_rs::Ticker::new(&client, "META");
    let _ = t5.calendar().await;
}
