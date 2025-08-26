use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
#[ignore]
async fn live_esg_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = YfClient::builder().build().unwrap();
    // Use a ticker known to have ESG data.
    let ticker = Ticker::new(client, "MSFT");

    // This will record `tests/fixtures/esg_api_MSFT.json` when YF_RECORD=1
    let esg = ticker.sustainability().await.unwrap();

    if !crate::common::is_recording() {
        // Basic sanity checks when running in live-only mode.
        // ESG data can sometimes be unavailable, so we check that at least one score is present.
        assert!(
            esg.total_esg.is_some()
                || esg.environment_score.is_some()
                || esg.social_score.is_some()
                || esg.governance_score.is_some(),
            "Expected at least one ESG score to be present for MSFT"
        );
        // Check a controversial flag that is usually stable
        assert!(
            !esg.involvement.tobacco,
            "Expected MSFT to not be involved in tobacco"
        );
    }
}
