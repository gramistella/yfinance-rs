use crate::common;
use httpmock::Method::GET;
use url::Url;
use yfinance_rs::{HistoryBuilder, Interval, YfClient};

#[tokio::test]
async fn history_allows_intraday_interval() {
    let server = common::setup_server();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "5m")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    // Only checking query params wiring; body content comes from fixture
    let _ = HistoryBuilder::new(&client, "AAPL")
        .interval(Interval::I5m)
        .fetch()
        .await
        .unwrap();

    mock.assert();
}