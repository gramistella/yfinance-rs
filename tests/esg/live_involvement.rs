use std::collections::HashSet;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_esg_involvement_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::new(&client, "MSFT");

    let summary = ticker.sustainability().await.unwrap();

    if !crate::common::is_recording() {
        // In live mode assert that involvement categories are unique and stable type-wise
        let cats: HashSet<String> = summary
            .involvement
            .iter()
            .map(|i| i.category.clone())
            .collect();
        assert_eq!(
            cats.len(),
            summary.involvement.len(),
            "involvement categories should be unique"
        );
        // If provider returns flags, we expect either empty or a small set; do a loose upper bound
        assert!(
            summary.involvement.len() <= 16,
            "unexpectedly high number of involvement categories"
        );
    }
}
