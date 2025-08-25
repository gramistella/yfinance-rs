use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{Range, Ticker, YfClient};

fn meta_body() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "meta": { "timezone":"America/New_York", "gmtoffset": -14400 },
            "timestamp": [],
            "indicators": {
              "quote":[{ "open":[], "high":[], "low":[], "close":[], "volume":[] }],
              "adjclose":[{ "adjclose":[] }]
            }
          }
        ],
        "error": null
      }
    }"#
    .to_string()
}

#[tokio::test]
async fn get_history_metadata_returns_timezone() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/MSFT")
            .query_param("range", "1d")
            .query_param("interval", "1d")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(meta_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let t = Ticker::new(client, "MSFT").unwrap();
    let meta = t.get_history_metadata(Some(Range::D1)).await.unwrap();

    mock.assert();
    let m = meta.expect("meta should be Some");
    assert_eq!(m.timezone.as_deref(), Some("America/New_York"));
    assert_eq!(m.gmtoffset, Some(-14400));
}
