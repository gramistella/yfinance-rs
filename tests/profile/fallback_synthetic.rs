use crate::common;
use httpmock::Method::GET;
use url::Url;
use yfinance_rs::{ApiPreference, Profile, YfClient};

#[tokio::test]
async fn api_then_scrape_fallback_on_other_error() {
    let server = common::setup_server();
    let sym = "AAPL";

    // API returns a generic error (not "Invalid Crumb")
    let api_err = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", sym));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":null,"error":{"description":"Something broke"}}}"#);
    });

    // Scrape path gets used instead
    let scrape = common::mock_profile_scrape(&server, sym);

    let mut client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .base_quote(Url::parse(&format!("{}/quote/", server.base_url())).unwrap())
        .api_preference(ApiPreference::ApiThenScrape)
        .preauth("cookie", "crumb")
        .build()
        .unwrap();

    let p = Profile::load(&mut client, sym).await.unwrap();
    api_err.assert();
    scrape.assert();

    match p {
        Profile::Company(c) => assert_eq!(c.name, "Apple Inc."),
        _ => panic!("expected Company"),
    }
}
