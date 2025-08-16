use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{HistoryBuilder, Interval, YfClient};

#[tokio::test]
async fn history_auto_adjust_uses_splits_when_adjclose_missing() {
    let server = MockServer::start();

    // No "adjclose" block; only a 2-for-1 split at the second timestamp.
    let body = r#"{
      "chart":{
        "result":[
          {
            "timestamp":[1000,2000],
            "indicators":{
              "quote":[{
                "open":[100.0,100.0],
                "high":[101.0,101.0],
                "low":[ 99.0, 99.0],
                "close":[100.0,100.0],
                "volume":[10,10]
              }],
              "adjclose":[]
            },
            "events": {
              "splits": {
                "2000": { "date": 2000, "numerator": 2, "denominator": 1 }
              }
            }
          }
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/TEST");
        then.status(200).header("content-type","application/json").body(body);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build().unwrap();

    let resp = HistoryBuilder::new(&client, "TEST")
        .interval(Interval::D1)
        .auto_adjust(true)
        .fetch_full().await.unwrap();

    mock.assert();

    assert!(resp.adjusted);
    assert_eq!(resp.candles.len(), 2);
    // First bar should be split-adjusted (0.5x); second bar unchanged.
    assert!((resp.candles[0].close - 50.0).abs() < 1e-9);
    assert!((resp.candles[1].close - 100.0).abs() < 1e-9);
    // Volume before split should be multiplied by 2 and rounded.
    assert_eq!(resp.candles[0].volume, Some(20));
}
