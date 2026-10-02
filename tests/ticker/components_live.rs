use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_index_components_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = YfClient::builder().build().unwrap();

    // Records `tests/fixtures/components_api_components_^FCHI.json` when YF_RECORD=1
    let components = Ticker::new(&client, "^FCHI").components().await.unwrap();

    if !crate::common::is_recording() {
        assert!(
            components.len() >= 30,
            "CAC 40 should list its members, got {components:?}"
        );
        assert!(components.iter().any(|s| s == "MC.PA"));
    }
}

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_equity_has_no_components_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = YfClient::builder().build().unwrap();

    // Records `tests/fixtures/components_api_components_AAPL.json` when YF_RECORD=1
    let components = Ticker::new(&client, "AAPL").components().await.unwrap();

    if !crate::common::is_recording() {
        assert!(components.is_empty(), "got {components:?}");
    }
}
