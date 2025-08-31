use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{SearchBuilder, YfClient};

fn fixture(endpoint: &str, key: &str) -> String {
    crate::common::fixture(endpoint, key, "json")
}

#[tokio::test]
async fn offline_search_uses_recorded_fixture() {
    // Query we'll use for fixture key
    let query = "apple";
    let server = MockServer::start();

    // Mock Yahoo /v1/finance/search with expected params
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/finance/search")
            .query_param("q", query)
            .query_param("quotesCount", "10")
            .query_param("newsCount", "0")
            .query_param("listsCount", "0");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("search_v1", query));
    });

    let client = YfClient::builder().build().unwrap();

    let resp = SearchBuilder::new(&client, query)
        .search_base(Url::parse(&format!("{}/v1/finance/search", server.base_url())).unwrap())
        .fetch()
        .await
        .unwrap();

    mock.assert();
    // Count should reflect the number of quotes in the fixture
    assert_eq!(
        resp.count,
        Some(resp.quotes.len() as u32),
        "record with YF_RECORD=1 first"
    );
    assert!(!resp.quotes.is_empty(), "record with YF_RECORD=1 first");
    assert!(
        resp.quotes
            .iter()
            .any(|q| q.symbol == "AAPL" || q.shortname.as_deref() == Some("Apple Inc."))
    );
}
