// tests/holders/live.rs

use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
#[ignore]
async fn live_holders_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = YfClient::builder().build().unwrap();
    // Use a major stock that is guaranteed to have all types of holder data.
    let t = Ticker::new(client, "AAPL");

    // Call all methods to ensure a complete fixture is recorded if YF_RECORD=1
    let major = t.major_holders().await.unwrap();
    let institutional = t.institutional_holders().await.unwrap();
    let mutual_fund = t.mutual_fund_holders().await.unwrap();
    let insider_trans = t.insider_transactions().await.unwrap();
    let insider_roster = t.insider_roster_holders().await.unwrap();
    let net_purchase = t.net_share_purchase_activity().await.unwrap();

    // If just running live (not recording), do some basic sanity checks.
    if !crate::common::is_recording() {
        assert!(!major.is_empty(), "expected major holders");
        assert!(!institutional.is_empty(), "expected institutional holders");
        assert!(!mutual_fund.is_empty(), "expected mutual fund holders");
        assert!(!insider_roster.is_empty(), "expected insider roster");
        assert!(net_purchase.is_some(), "expected net purchase activity");
        // Insider transactions can often be empty, so we don't assert on it.
        let _ = insider_trans;
    }
}
