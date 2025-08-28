use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{HistoryBuilder, Range, YfClient};

fn minimal_ok_body() -> String {
    r#"{
      "chart": {
        "result": [{
          "meta": {"timezone":"America/New_York","gmtoffset":-14400},
          "timestamp": [],
          "indicators": { "quote":[{ "open":[], "high":[], "low":[], "close":[], "volume":[] }], "adjclose":[{"adjclose":[]}] }
        }],
        "error": null
      }
    }"#.to_string()
}

#[tokio::test]
async fn history_range_1d_ytd_10y() {
    let server = MockServer::start();

    let m1d = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "1d")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(minimal_ok_body());
    });

    let mytd = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "ytd")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(minimal_ok_body());
    });

    let m10y = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "10y")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(minimal_ok_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let _ = HistoryBuilder::new(&client, "AAPL")
        .range(Range::D1)
        .fetch()
        .await
        .unwrap();
    let _ = HistoryBuilder::new(&client, "AAPL")
        .range(Range::Ytd)
        .fetch()
        .await
        .unwrap();
    let _ = HistoryBuilder::new(&client, "AAPL")
        .range(Range::Y10)
        .fetch()
        .await
        .unwrap();

    m1d.assert();
    mytd.assert();
    m10y.assert();
}
