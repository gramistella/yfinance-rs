use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{Ticker, YfClient};

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

#[tokio::test]
async fn offline_esg_uses_recorded_fixture() {
    let sym = "MSFT";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "esgScores")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("esg_api_esgScores", sym));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, sym);
    let esg = ticker.sustainability().await.unwrap();

    mock.assert();

    // These assertions depend on the contents of the recorded fixture.
    // If you re-record the fixture, you may need to update these values.
    assert!(
        esg.total_esg.is_some(),
        "total_esg should be present in the fixture. Did you run `just test-record esg`?"
    );
    assert!(!esg.involvement.tobacco, "tobacco flag should be false");
    assert!(
        !esg.involvement.controversial_weapons,
        "controversial_weapons flag should be false"
    );
}
