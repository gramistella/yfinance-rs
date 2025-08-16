use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;

#[tokio::test]
async fn history_returns_status_error_on_non_2xx() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/FAIL");
        then.status(500).body("oops");
    });

    let client = yfinance_rs::YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build().unwrap();

    let err = yfinance_rs::HistoryBuilder::new(&client, "FAIL").fetch().await.unwrap_err();
    mock.assert();

    match err {
        yfinance_rs::YfError::Status { status, url } => {
            assert_eq!(status, 500);
            assert!(url.contains("/v8/finance/chart/FAIL"));
        }
        other => panic!("expected Status error, got {other:?}"),
    }
}
