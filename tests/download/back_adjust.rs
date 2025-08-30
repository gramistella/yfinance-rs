use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::YfClient;

#[tokio::test]
async fn download_back_adjust_sets_close_to_raw() {
    // One day has adjclose=50 while raw close=100 (e.g., dividend/split adjustment)
    let body = r#"{
      "chart": {
        "result": [{
          "timestamp":[1000,2000],
          "indicators":{
            "quote":[{ "open":[100.0,100.0], "high":[105.0,105.0], "low":[95.0,95.0], "close":[100.0,100.0], "volume":[1000,1000] }],
            "adjclose":[{ "adjclose":[50.0,100.0] }]
          }
        }],
        "error": null
      }
    }"#;

    let server = MockServer::start();
    let sym = "TEST";

    let mock = server.mock(|when, then| {
        when.method(GET).path(format!("/v8/finance/chart/{sym}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let res = yfinance_rs::DownloadBuilder::new(&client)
        .symbols([sym])
        .back_adjust(true)
        .run()
        .await
        .unwrap();

    mock.assert();

    let s = res.series.get(sym).expect("symbol data");
    // first bar got 50% adjustment factor; OHLC adjusted => open≈50, high≈52.5, low≈47.5
    assert!((s[0].open - 50.0).abs() < 1e-9);
    // back_adjust keeps raw Close
    assert!((s[0].close - 100.0).abs() < 1e-9);
    // second bar unchanged
    assert!((s[1].open - 100.0).abs() < 1e-9);
    assert!((s[1].close - 100.0).abs() < 1e-9);
}
