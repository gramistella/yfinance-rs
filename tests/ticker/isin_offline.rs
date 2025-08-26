use crate::common::{fixture, mock_quote_v7};
use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
async fn offline_isin_happy_path() {
    let server = MockServer::start();
    let sym = "AAPL";

    // 1. Mock the Yahoo quote endpoint to provide the shortName for the search query.
    // This will use the `quote_v7_AAPL.json` fixture.
    let quote_mock = mock_quote_v7(&server, sym);

    // 2. Mock the Business Insider endpoint.
    // This will use the `isin_search_AAPL.json` fixture recorded by the live test.
    let isin_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/ajax/SearchController_Suggest")
            .query_param_exists("query");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("isin_search", sym, "json"));
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
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

    quote_mock.assert();
    isin_mock.assert();
    assert_eq!(
        isin,
        Some("US0378331005".to_string()),
        "ISIN not parsed from fixture. Did you run `just test-record ticker` first?"
    );
}