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

    // Ensure at least one ESG component exists in the summary
    let has_any = esg
        .scores
        .as_ref()
        .is_some_and(|s| s.environmental.is_some() || s.social.is_some() || s.governance.is_some());
    assert!(
        has_any,
        "At least one ESG component score should be present. Did you run `just test-record esg`?"
    );
}
