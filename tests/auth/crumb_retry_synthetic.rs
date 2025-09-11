use crate::common;
use httpmock::Method::GET;
use url::Url;
use yfinance_rs::YfClient;
use paft::fundamentals::Profile;

#[tokio::test]
async fn api_fetches_cookie_and_crumb_first() {
    let server = common::setup_server();
    // 1) cookie + crumb
    let (cookie_mock, crumb_mock) = common::mock_cookie_crumb(&server);

    // 2) API (uses the crumb we just fetched)
    let sym = "AAPL";
    let api = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", "crumb-value");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture(
                "profile_api_assetProfile-quoteType-fundProfile",
                sym,
                "json",
            ));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        .build()
        .unwrap();

    let p = yfinance_rs::profile::load_profile(&client, sym)
        .await
        .unwrap();
    api.assert();
    cookie_mock.assert();
    crumb_mock.assert();

    match p {
        Profile::Company(c) => assert_eq!(c.name, "Apple Inc."),
        _ => panic!("expected Company"),
    }
}

#[tokio::test]
async fn api_retries_on_invalid_crumb_then_succeeds() {
    let server = common::setup_server();

    // start with a stale crumb so first call fails
    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        ._preauth("cookie", "stale-crumb")
        .build()
        .unwrap();

    // first API call returns "Invalid Crumb"
    let sym = "AAPL";
    let invalid = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", "stale-crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":null,"error":{"description":"Invalid Crumb"}}}"#);
    });

    // crumb refresh happens here
    let (cookie_mock, crumb_mock) = common::mock_cookie_crumb(&server);

    // second API call with fresh crumb
    let ok = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", "crumb-value");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture(
                "profile_api_assetProfile-quoteType-fundProfile",
                sym,
                "json",
            ));
    });

    let p = yfinance_rs::profile::load_profile(&client, sym)
        .await
        .unwrap();
    invalid.assert();
    cookie_mock.assert();
    crumb_mock.assert();
    ok.assert();

    match p {
        Profile::Company(c) => assert_eq!(c.name, "Apple Inc."),
        _ => panic!("expected Company"),
    }
}
