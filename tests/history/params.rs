use crate::common;
use httpmock::Method::GET;
use url::Url;
use yfinance_rs::{HistoryBuilder, Range, YfClient};

#[tokio::test]
async fn history_has_expected_query_params() {
    let server = common::setup_server();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let _ = HistoryBuilder::new(&client, "AAPL")
        .range(Range::M6)
        .fetch()
        .await
        .unwrap();

    mock.assert();
}
