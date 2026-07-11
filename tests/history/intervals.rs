use crate::common;
use httpmock::Method::GET;
use url::Url;
use yfinance_rs::core::{Interval, Range};
use yfinance_rs::{HistoryBuilder, YfClient};

#[tokio::test]
async fn history_allows_intraday_interval() {
    let server = common::setup_server();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/IBM")
            .query_param("range", "5d")
            .query_param("interval", "5m")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "IBM", "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = HistoryBuilder::new(&client, "IBM")
        .range(Range::D5)
        .interval(Interval::I5m)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    assert!(bars.len() > 100, "expected recorded five-minute candles");
}
