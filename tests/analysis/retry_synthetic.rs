use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient};

#[tokio::test]
async fn analysis_invalid_crumb_then_retry_succeeds() {
    let server = MockServer::start();
    let sym = "AAPL";

    let first = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "recommendationTrend,financialData")
            .query_param("crumb", "stale");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":null,"error":{"description":"Invalid Crumb"}}}"#);
    });

    let cookie = server.mock(|when, then| {
        when.method(GET).path("/consent");
        then.status(200).header(
            "set-cookie",
            "A=B; Max-Age=315360000; Domain=.yahoo.com; Path=/; Secure; SameSite=None",
        );
    });
    let crumb = server.mock(|when, then| {
        when.method(GET).path("/v1/test/getcrumb");
        then.status(200).body("fresh");
    });

    let ok = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "recommendationTrend,financialData")
            .query_param("crumb", "fresh");
        then.status(200)
            .header("content-type","application/json")
            .body(r#"{
              "quoteSummary": {
                "result": [{
                  "recommendationTrend": { "trend": [] },
                  "financialData": { "recommendationMean": { "raw": 2.5 }, "recommendationKey": "buy" }
                }],
                "error": null
              }
            }"#);
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", "stale")
        .build()
        .unwrap();

    let t = Ticker::new(&client, sym);
    let s = t.recommendations_summary().await.unwrap();

    first.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();

    assert_eq!(s.mean, Some(2.5));
    assert_eq!(s.mean_key.as_deref(), Some("buy"));
}
