use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_info_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::new(&client, "MSFT");

    // This will trigger all the API calls needed for .info()
    // and record fixtures if YF_RECORD=1 is set.
    // Note: The concurrent nature of .info() means the `analysis_api_MSFT.json`
    // fixture will contain the modules from whichever analysis call finishes last.
    // The offline test is designed to handle this.
    let info = ticker.info().await.unwrap();

    if !crate::common::is_recording() {
        // Basic sanity checks for live mode
        match info.instrument.id() {
            paft::domain::IdentifierScheme::Security(s) => assert_eq!(s.symbol.as_str(), "MSFT"),
            paft::domain::IdentifierScheme::Prediction(_) => {
                panic!("unexpected instrument identifier scheme")
            }
        }
        assert!(info.last.is_some(), "Expected a market price for MSFT");
    }
}
