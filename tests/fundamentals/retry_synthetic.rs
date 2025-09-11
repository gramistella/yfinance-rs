use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient};

#[tokio::test]
async fn fundamentals_invalid_crumb_then_retry_succeeds() {
    let server = MockServer::start();
    let sym = "AAPL";

    // first call with stale crumb -> Invalid Crumb
    let first = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "stale");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":null,"error":{"description":"Invalid Crumb"}}}"#);
    });

    // cookie + crumb refresh endpoints
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

    // second call with fresh crumb returns minimal earnings payload
    let ok = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "fresh");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
              "quoteSummary": {
                "result": [{
                  "earnings": {
                    "financialsChart": {
                      "yearly": [],
                      "quarterly": []
                    },
                    "earningsChart": {
                      "quarterly": []
                    }
                  }
                }],
                "error": null
              }
            }"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        ._api_preference(ApiPreference::ApiOnly)
        ._preauth("cookie", "stale")
        .build()
        .unwrap();

    let t = Ticker::new(&client, sym);
    let e = t.earnings().await.unwrap();

    first.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();

    // We just verify it returns a valid (possibly empty) structure
    assert!(e.yearly.is_empty() && e.quarterly.is_empty() && e.quarterly_eps.is_empty());
}
