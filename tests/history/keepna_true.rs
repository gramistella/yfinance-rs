use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{HistoryBuilder, YfClient};

#[tokio::test]
async fn history_keepna_preserves_null_rows() {
    let server = MockServer::start();

    let body = r#"{
      "chart":{"result":[{"timestamp":[1,2],
        "indicators":{"quote":[{
          "open":[100.0,null],
          "high":[101.0,null],
          "low":[ 99.0,null],
          "close":[100.5,null],
          "volume":[1000,2000]
        }]}}],"error":null}
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/AAPL");
        then.status(200).header("content-type","application/json").body(body);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build().unwrap();

    let bars = HistoryBuilder::new(&client, "AAPL")
        .keepna(true)
        .fetch().await.unwrap();

    mock.assert();

    assert_eq!(bars.len(), 2, "second row kept with NaNs");
    assert!(bars[1].open.is_nan() && bars[1].close.is_nan());
    assert_eq!(bars[1].volume, Some(2000));
}
