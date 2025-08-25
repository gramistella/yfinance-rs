use httpmock::Method::GET;
use httpmock::MockServer;
use std::time::Duration;
use url::Url;
use yfinance_rs::{core::client::{Backoff, RetryConfig}, HistoryBuilder, YfClient, YfError};

#[tokio::test]
async fn history_retries_on_persistent_5xx() {
    let server = MockServer::start();
    let sym = "RETRY";

    // This single mock will persistently fail, allowing us to count the retries.
    let fail_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v8/finance/chart/{}", sym))
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("events", "div|split")
            .query_param("includePrePost", "false");
        then.status(503).body("Service Unavailable");
    });

    let max_retries = 3;
    let mut client_retry_config = RetryConfig::default();
    client_retry_config.backoff = Backoff::Fixed(Duration::from_millis(1)); // Minimal delay for fast tests
    client_retry_config.max_retries = max_retries;

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .retry_config(client_retry_config)
        .build()
        .unwrap();

    // We expect this to fail, so we match on the Err variant.
    let result = HistoryBuilder::new(&client, sym).fetch().await;

    // The mock should be hit 1 (initial) + 3 (retries) = 4 times.
    fail_mock.assert_hits((1 + max_retries) as usize);

    // Assert that the final result is the expected error.
    match result {
        Err(YfError::Status { status, .. }) => {
            assert_eq!(status, 503);
        }
        _ => panic!("Expected a Status error after all retries failed."),
    }
}
