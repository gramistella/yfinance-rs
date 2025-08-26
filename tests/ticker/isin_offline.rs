use crate::common::fixture;
use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
async fn offline_isin_happy_path() {
    let server = MockServer::start();
    let sym = "AAPL";

    let isin_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/ajax/SearchController_Suggest")
            .query_param("query", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("isin_search", sym, "json"));
    });

    let client = YfClient::builder()
        .base_insider_search(
            Url::parse(&format!(
                "{}/ajax/SearchController_Suggest",
                server.base_url()
            ))
            .unwrap(),
        )
        .build()
        .unwrap();

    let ticker = Ticker::new(client, sym);
    let isin = ticker.isin().await.unwrap();

    isin_mock.assert();
    assert_eq!(
        isin,
        Some("US0378331005".to_string()),
        "ISIN not parsed from fixture. Did you run `just test-record ticker` first?"
    );
}
