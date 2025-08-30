use crate::common::setup_server;
use httpmock::Method::GET;
use url::Url;
use yfinance_rs::{ApiPreference, Profile, YfClient};

#[tokio::test]
async fn profile_api_company_happy() {
    let server = setup_server();
    let sym = "AAPL";
    let crumb = "test-crumb";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture(
                "profile_api_assetProfile-quoteType-fundProfile",
                sym,
                "json",
            ));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", crumb)
        .build()
        .unwrap();

    let prof = Profile::load(&client, sym).await.unwrap();
    mock.assert();

    match prof {
        Profile::Company(c) => {
            assert_eq!(c.name, "Apple Inc.");
            assert_eq!(c.sector.as_deref(), Some("Technology"));
            assert_eq!(c.industry.as_deref(), Some("Consumer Electronics"));
            assert_eq!(c.website.as_deref(), Some("https://www.apple.com"));
        }
        _ => panic!("expected Company"),
    }
}

#[tokio::test]
async fn profile_api_fund_happy() {
    let server = setup_server();
    let sym = "QQQ";
    let crumb = "test-crumb";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture(
                "profile_api_assetProfile-quoteType-fundProfile",
                sym,
                "json",
            ));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", crumb)
        .build()
        .unwrap();

    let prof = Profile::load(&client, sym).await.unwrap();
    mock.assert();

    match prof {
        Profile::Fund(f) => {
            assert_eq!(f.name, "Invesco QQQ Trust");
            assert_eq!(f.family.as_deref(), Some("Invesco"));
            assert_eq!(f.kind, "Exchange Traded Fund");
        }
        _ => panic!("expected Fund"),
    }
}
