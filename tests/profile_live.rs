#[tokio::test]
#[ignore] // opt-in: run with `YF_LIVE=1 cargo test -- --ignored`
async fn live_profile_company() {
    if std::env::var("YF_LIVE").ok().as_deref() != Some("1") {
        return;
    }

    let mut client = yfinance_rs::YfClient::builder().build().unwrap();
    let prof = yfinance_rs::Profile::load(&mut client, "AAPL")
        .await
        .unwrap();
    match prof {
        yfinance_rs::Profile::Company(c) => assert_eq!(c.name, "Apple Inc."),
        _ => panic!("expected Company"),
    }
}
