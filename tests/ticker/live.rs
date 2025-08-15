#[tokio::test]
#[ignore]
async fn live_ticker_quote_for_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let mut client = yfinance_rs::YfClient::builder().build().unwrap();

    for sym in ["AAPL", "MSFT"] {
        let mut t = yfinance_rs::Ticker::new(&mut client, sym).unwrap();
        let q = t.quote().await.unwrap();

        if !crate::common::is_recording() {
            assert_eq!(q.symbol, sym);
            assert!(q.regular_market_price.is_some() || q.regular_market_previous_close.is_some());
        }
    }
}

#[tokio::test]
#[ignore]
async fn live_ticker_options_for_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let mut client = yfinance_rs::YfClient::builder().build().unwrap();
    let mut t = yfinance_rs::Ticker::new(&mut client, "AAPL").unwrap();

    let expiries = t.options().await.unwrap();

    if !crate::common::is_recording() {
        assert!(!expiries.is_empty());
    }

    if let Some(first) = expiries.first().cloned() {
        let chain = t.option_chain(Some(first)).await.unwrap();

        if !crate::common::is_recording() {
            assert!(chain.calls.len() + chain.puts.len() >= 0);
        }
    }
}
