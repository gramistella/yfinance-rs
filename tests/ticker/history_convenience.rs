use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::YfClient;
use yfinance_rs::core::conversions::*;
use yfinance_rs::core::{Interval, Range};

fn minimal_ok_body() -> String {
    r#"{
      "chart": {
        "result": [{
          "meta": {"timezone":"America/New_York","gmtoffset":-14400},
          "timestamp": [1000],
          "indicators": {
            "quote":[{ "open":[100.0], "high":[101.0], "low":[99.0], "close":[100.5], "volume":[1000] }],
            "adjclose":[{ "adjclose":[100.5] }]
          }
        }],
        "error": null
      }
    }"#.to_string()
}

#[tokio::test]
async fn ticker_history_convenience_builds_expected_query() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "ytd")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(minimal_ok_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let ticker = yfinance_rs::Ticker::new(&client, "AAPL");
    let bars = ticker
        .history(Some(Range::Ytd), Some(Interval::D1), false)
        .await
        .unwrap();

    mock.assert();
    assert_eq!(bars.len(), 1);
    assert!((money_to_f64(&bars[0].close) - 100.5).abs() < 1e-9);
}
