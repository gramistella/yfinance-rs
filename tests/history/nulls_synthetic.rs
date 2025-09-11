use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{HistoryBuilder, YfClient};
use yfinance_rs::core::conversions::*;

#[tokio::test]
async fn history_skips_points_with_null_ohlc() {
    let server = MockServer::start();

    // Minimal chart payload: first point valid, second has open=null so must be skipped
    let body = r#"{
      "chart":{"result":[{"timestamp":[1704067200,1704153600],
        "indicators":{"quote":[{
          "open":[100.0,null],
          "high":[101.0,102.0],
          "low":[99.0,100.0],
          "close":[100.5,101.5],
          "volume":[1000000,1100000]
        }]}}],"error":null}
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/AAPL");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = HistoryBuilder::new(&client, "AAPL").fetch().await.unwrap();
    mock.assert();
    assert_eq!(
        bars.len(),
        1,
        "second point with null open should be filtered out"
    );
    assert_eq!(bars[0].open, f64_to_money_usd(100.0));
    assert_eq!(bars[0].close, f64_to_money_usd(100.5));
    assert_eq!(bars[0].volume, Some(1_000_000));
}
