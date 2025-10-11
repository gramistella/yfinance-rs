use httpmock::{Method::GET, MockServer};
use std::time::Duration;
use url::Url;
use yfinance_rs::{HistoryBuilder, YfClient, core::client::CacheMode};

#[tokio::test]
async fn history_serves_from_cache_on_second_call() {
    let server = MockServer::start();
    let sym = "CACHE";

    // This mock only expects to be called ONCE.
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v8/finance/chart/{sym}"))
            .query_param("range", "6mo")
            .query_param("interval", "1d");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("history_chart", "AAPL", "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .cache_ttl(Duration::from_secs(10)) // Enable caching
        .build()
        .unwrap();

    let builder = HistoryBuilder::new(&client, sym);

    // First call, should hit the network
    let bars1 = builder.clone().fetch().await.unwrap();
    mock.assert(); // Verifies the mock was called exactly once

    // Second call, should be served from cache
    let bars2 = builder.clone().fetch().await.unwrap();

    // Verify again. The hit count should still be 1.
    mock.assert();

    assert_eq!(bars1.len(), bars2.len());
    assert_eq!(bars1[0], bars2[0]);
}

#[tokio::test]
async fn history_cache_refresh_bypasses_cache_get_but_updates_cache() {
    let server = MockServer::start();
    let sym = "REFRESH";

    // This mock expects to be called TWICE.
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v8/finance/chart/{sym}"))
            .query_param("range", "6mo")
            .query_param("interval", "1d");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("history_chart", "MSFT", "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .cache_ttl(Duration::from_secs(10)) // Enable caching
        .build()
        .unwrap();

    let builder = HistoryBuilder::new(&client, sym);

    // First call, hits network and populates cache
    let _ = builder.clone().fetch().await.unwrap();
    mock.assert_calls(1);

    // Second call with CacheMode::Refresh, should hit network again
    let _ = builder
        .clone()
        .cache_mode(CacheMode::Refresh)
        .fetch()
        .await
        .unwrap();
    mock.assert_calls(2);

    // Third call with default CacheMode::Use, should now be served from cache
    let _ = builder.clone().fetch().await.unwrap();
    // The hit count should NOT increase, so we assert it's still 2.
    mock.assert_calls(2);
}
