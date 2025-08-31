#[tokio::test]
#[ignore]
async fn live_ticker_quote_for_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();

    for sym in ["AAPL", "MSFT"] {
        let t = yfinance_rs::Ticker::new(&client, sym);
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

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let t = yfinance_rs::Ticker::new(&client, "AAPL");

    let expiries = t.options().await.unwrap();

    if !crate::common::is_recording() {
        // In live mode (non-recording), we expect Yahoo to return at least one expiry.
        assert!(!expiries.is_empty());
    }

    if let Some(first) = expiries.first().cloned() {
        let chain = t.option_chain(Some(first)).await.unwrap();

        if !crate::common::is_recording() {
            // Instead of a useless `>= 0` check on usize, ensure the chain is coherent:
            // every returned contract (if any) must match the requested expiration.
            assert!(
                chain
                    .calls
                    .iter()
                    .chain(chain.puts.iter())
                    .all(|c| c.expiration == first),
                "all option contracts should match the requested expiration"
            );
        }
    }
}

#[tokio::test]
#[ignore]
async fn live_ticker_shares_for_record() {
    if !crate::common::is_recording() {
        return;
    }
    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let t = yfinance_rs::Ticker::new(&client, "MSFT");
    let _ = t.shares().await;
    let _ = t.quarterly_shares().await;
}

#[tokio::test]
#[ignore]
async fn live_ticker_capital_gains_for_record() {
    if !crate::common::is_recording() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let t = yfinance_rs::Ticker::new(&client, "VFINX");
    let _ = t.capital_gains(None).await.unwrap();
}
