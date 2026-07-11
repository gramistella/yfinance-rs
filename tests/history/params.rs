use crate::common;
use httpmock::Method::GET;
use url::Url;
use yfinance_rs::core::{Interval, Range};
use yfinance_rs::{HistoryBuilder, YfClient};

#[tokio::test]
async fn history_has_expected_query_params() {
    let server = common::setup_server();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
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

#[tokio::test]
async fn history_max_daily_uses_explicit_window_and_preserves_granularity() {
    let server = common::setup_server();

    let fixture = common::fixture("history_chart", "VFINX", "json");
    let raw: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    assert_eq!(
        raw.pointer("/chart/result/0/meta/dataGranularity")
            .and_then(serde_json::Value::as_str),
        Some("1d"),
        "the recorded Max/D1 fixture must contain Yahoo's daily response"
    );

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/VFINX")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains")
            .query_param_missing("range")
            .is_true(common::is_daily_max_period_query);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let history = HistoryBuilder::new(&client, "VFINX")
        .range(Range::Max)
        .interval(Interval::D1)
        .fetch_full()
        .await
        .unwrap();

    mock.assert();
    assert!(
        history.candles.len() > 5_000,
        "expected full daily history, got only {} candles",
        history.candles.len()
    );
}
