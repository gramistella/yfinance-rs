use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::YfClient;

#[tokio::test]
async fn download_keepna_and_rounding() {
    // Well-formed JSON: adjclose belongs inside indicators, and braces are balanced.
    let body = r#"{
      "chart": {
        "result": [{
          "timestamp": [10, 20, 30],
          "indicators": {
            "quote": [{
              "open":  [100.001, null,  99.994],
              "high":  [101.009, null, 100.006],
              "low":   [ 99.001, null,  98.994],
              "close": [100.499, null,  99.996],
              "volume":[   1000,  2000,    3000]
            }],
            "adjclose": [{
              "adjclose": [100.499, null, 99.996]
            }]
          }
        }],
        "error": null
      }
    }"#;

    let server = MockServer::start();
    let sym = "AAPL";

    let mock = server.mock(|when, then| {
        when.method(GET).path(format!("/v8/finance/chart/{}", sym));
        then.status(200).header("content-type","application/json").body(body);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build().unwrap();

    let res = yfinance_rs::DownloadBuilder::new(&client)
        .symbols([sym])
        .keepna(true)
        .rounding(true)
        .run().await.unwrap();

    mock.assert();

    let v = res.series.get(sym).unwrap();
    assert_eq!(v.len(), 3, "kept NA row");
    // row 1 rounded to 2dp
    assert!((v[0].open  - 100.00).abs() < 1e-9);
    assert!((v[0].high  - 101.01).abs() < 1e-9);
    assert!((v[0].low   -  99.00).abs() < 1e-9);
    assert!((v[0].close - 100.50).abs() < 1e-9);
    // NA row should have NaN OHLC preserved
    assert!(v[1].open.is_nan() && v[1].high.is_nan() && v[1].low.is_nan() && v[1].close.is_nan());
}
