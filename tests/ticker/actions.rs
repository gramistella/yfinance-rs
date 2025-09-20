use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::core::Range;
use yfinance_rs::{Ticker, YfClient};

fn body_with_actions() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "timestamp":[1000,2000,3000],
            "indicators":{
              "quote":[{
                "open":[100.0,100.0,100.0],
                "high":[101.0,101.0,101.0],
                "low":[99.0,99.0,99.0],
                "close":[100.0,100.0,100.0],
                "volume":[10,10,10]
              }],
              "adjclose":[{"adjclose":[50.0,100.0,99.0]}]
            },
            "events":{
              "splits":{
                "2000":{"date":2000,"numerator":2,"denominator":1}
              },
              "dividends":{
                "3000":{"date":3000,"amount":1.0}
              }
            }
          }
        ],
        "error":null
      }
    }"#
    .to_string()
}

#[tokio::test]
async fn ticker_actions_dividends_splits() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/TEST")
            .query_param("range", "max")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(body_with_actions());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let t = Ticker::new(&client, "TEST");

    let acts = t.actions(None).await.unwrap();
    mock.assert();

    assert_eq!(acts.len(), 2);
    let divs = t.dividends(Some(Range::Max)).await.unwrap();
    assert_eq!(divs, vec![(3000, 1.0)]);
    let splits = t.splits(Some(Range::Max)).await.unwrap();
    assert_eq!(splits, vec![(2000, 2, 1)]);
}
