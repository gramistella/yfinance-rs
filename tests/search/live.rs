use yfinance_rs::{SearchBuilder, YfClient};

#[tokio::test]
#[ignore]
async fn live_search_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let mut client = YfClient::builder().build().unwrap();

    // Keep the query simple/stable so the recorded fixture is reusable.
    let query = "apple";

    let resp = SearchBuilder::new(&mut client, query)
        .unwrap()
        .fetch()
        .await
        .unwrap();

    if !crate::common::is_recording() {
        assert!(!resp.quotes.is_empty());
        // Heuristic: expect to see AAPL when searching "apple"
        let has_aapl = resp.quotes.iter().any(|q| q.symbol == "AAPL");
        assert!(has_aapl, "expected AAPL among search results for 'apple'");
    }
}
