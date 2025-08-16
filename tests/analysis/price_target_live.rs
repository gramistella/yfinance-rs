use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
#[ignore]
async fn live_price_target_smoke() {
    // Only run this when explicitly enabled
    if std::env::var("YF_LIVE").ok().as_deref() != Some("1") {
        return;
    }

    // Default client hits real Yahoo endpoints.
    let mut client = YfClient::builder().build().unwrap();

    // Pick a very liquid name that usually has coverage.
    let mut t = Ticker::new(&mut client, "AAPL").unwrap();
    let pt = t.analyst_price_target().await.unwrap();

    // Basic sanity: at least one of the fields should show up.
    assert!(
        pt.mean.is_some()
            || pt.high.is_some()
            || pt.low.is_some()
            || pt.number_of_analysts.is_some(),
        "expected at least one price target field to be present"
    );
}
