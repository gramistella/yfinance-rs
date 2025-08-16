#[tokio::test]
#[ignore]
async fn live_fundamentals_smoke() {
    // Mirrors the live/record pattern used elsewhere
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let mut client = yfinance_rs::YfClient::builder().build().unwrap();

    // Use distinct symbols to avoid fixture clobbering since endpoint name is the same
    // ("fundamentals_api_<SYMBOL>.json")
    // income (quarterly)
    {
        let mut t = yfinance_rs::Ticker::new(&mut client, "AAPL").unwrap();
        let _ = t.quarterly_income_stmt().await.unwrap();
    }
    // balance (annual)
    {
        let mut t = yfinance_rs::Ticker::new(&mut client, "MSFT").unwrap();
        let _ = t.balance_sheet().await.unwrap();
    }
    // cashflow (annual)
    {
        let mut t = yfinance_rs::Ticker::new(&mut client, "GOOGL").unwrap();
        let _ = t.cashflow().await.unwrap();
    }
    // earnings
    {
        let mut t = yfinance_rs::Ticker::new(&mut client, "AMZN").unwrap();
        let _ = t.earnings().await.unwrap();
    }
    // calendar
    {
        let mut t = yfinance_rs::Ticker::new(&mut client, "META").unwrap();
        let _ = t.calendar().await.unwrap();
    }

    if !crate::common::is_recording() {
        // If not recording, at least assert we got *some* data from live
        // (No strict expectations; shapes vary by company)
        let mut t = yfinance_rs::Ticker::new(&mut client, "AAPL").unwrap();
        let income = t.quarterly_income_stmt().await.unwrap();
        assert!(!income.is_empty());
    }
}

#[tokio::test]
#[ignore]
async fn live_fundamentals_for_record() {
    // Only run when actually recording; this populates fixtures for offline tests.
    if !crate::common::is_recording() {
        return;
    }

    let mut client = yfinance_rs::YfClient::builder().build().unwrap();

    let mut t1 = yfinance_rs::Ticker::new(&mut client, "AAPL").unwrap();
    let _ = t1.quarterly_income_stmt().await;

    let mut t2 = yfinance_rs::Ticker::new(&mut client, "MSFT").unwrap();
    let _ = t2.balance_sheet().await;

    let mut t3 = yfinance_rs::Ticker::new(&mut client, "GOOGL").unwrap();
    let _ = t3.cashflow().await;

    let mut t4 = yfinance_rs::Ticker::new(&mut client, "AMZN").unwrap();
    let _ = t4.earnings().await;

    let mut t5 = yfinance_rs::Ticker::new(&mut client, "META").unwrap();
    let _ = t5.calendar().await;
}
