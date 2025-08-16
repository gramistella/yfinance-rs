use httpmock::{Method::GET, MockServer};
use url::Url;

#[tokio::test]
async fn offline_history_uses_recorded_fixture() {
    fn fixture_dir() -> std::path::PathBuf {
        std::env::var("YF_FIXDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
            })
    }
    fn fixture(endpoint: &str, symbol: &str, ext: &str) -> String {
        let filename = format!("{}_{}.{}", endpoint, symbol, ext);
        let path = fixture_dir().join(&filename);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
    }

    let server = MockServer::start();
    let sym = "AAPL";

    let mock = server.mock(|when, then| {
        when.method(GET).path(format!("/v8/finance/chart/{}", sym));
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("history_chart", sym, "json"));
    });

    let client = yfinance_rs::YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = yfinance_rs::HistoryBuilder::new(&client, sym)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    assert!(!bars.is_empty(), "record with YF_RECORD=1 first");
}
