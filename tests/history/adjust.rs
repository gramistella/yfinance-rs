use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{Action, HistoryBuilder, Interval, YfClient};

#[tokio::test]
async fn history_auto_adjust_and_actions() {
    let server = MockServer::start();

    // Three days:
    // t1=1000 (before 2:1 split), t2=2000 (split date), t3=3000 (dividend date)
    // OHLC all ~100, volume = 10 each day
    // adjclose encodes: 0.5 factor on t1 (split), 1.0 on t2, 0.99 on t3 (dividend)
    let body = r#"{
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
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/TEST");
        then.status(200).header("content-type","application/json").body(body);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let resp = HistoryBuilder::new(&client, "TEST")
        .interval(Interval::D1)
        .auto_adjust(true)
        .fetch_full()
        .await
        .unwrap();

    mock.assert();

    assert!(resp.adjusted);
    assert_eq!(resp.candles.len(), 3);

    // t1 (1000): prices halved, volume doubled due to 2:1 split after this candle
    let c0 = &resp.candles[0];
    assert!((c0.open - 50.0).abs() < 1e-9);
    assert!((c0.high - 50.5).abs() < 1e-9);
    assert!((c0.low - 49.5).abs() < 1e-9);
    assert!((c0.close - 50.0).abs() < 1e-9);
    assert_eq!(c0.volume, Some(20));

    // t2 (2000): unchanged prices, unchanged volume
    let c1 = &resp.candles[1];
    assert!((c1.close - 100.0).abs() < 1e-9);
    assert_eq!(c1.volume, Some(10));

    // t3 (3000): dividend -> adjclose=99 => factor 0.99
    let c2 = &resp.candles[2];
    assert!((c2.close - 99.0).abs() < 1e-9);
    assert_eq!(c2.volume, Some(10));

    // actions parsed and sorted
    assert_eq!(resp.actions.len(), 2);
    assert!(matches!(resp.actions[0], Action::Split { ts, numerator:2, denominator:1 } if ts==2000));
    assert!(matches!(resp.actions[1], Action::Dividend { ts, amount } if ts==3000 && (amount-1.0).abs()<1e-9));
}