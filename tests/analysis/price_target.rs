use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient};

#[tokio::test]
async fn offline_price_target_happy() {
    let server = MockServer::start();
    let sym = "AAPL";

    let body = r#"{
      "quoteSummary": {
        "result": [{
          "financialData": {
            "targetMeanPrice": { "raw": 200.0 },
            "targetHighPrice": { "raw": 250.0 },
            "targetLowPrice":  { "raw": 150.0 },
            "numberOfAnalystOpinions": { "raw": 31 }
          }
        }],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", sym))
            .query_param("modules", "financialData")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(client, sym).unwrap();
    let pt = t.analyst_price_target().await.unwrap();

    mock.assert();

    assert_eq!(pt.mean, Some(200.0));
    assert_eq!(pt.high, Some(250.0));
    assert_eq!(pt.low, Some(150.0));
    assert_eq!(pt.number_of_analysts, Some(31));
}

#[tokio::test]
async fn price_target_invalid_crumb_then_retry_succeeds() {
    let server = MockServer::start();
    let sym = "MSFT";

    let first = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", sym))
            .query_param("modules", "financialData")
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
            .path(format!("/v10/finance/quoteSummary/{}", sym))
            .query_param("modules", "financialData")
            .query_param("crumb", "fresh");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
              "quoteSummary": {
                "result": [{
                  "financialData": {
                    "targetMeanPrice": { "raw": 123.45 },
                    "targetHighPrice": { "raw": 150.0 },
                    "targetLowPrice":  { "raw": 100.0 },
                    "numberOfAnalystOpinions": { "raw": 20 }
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
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", "stale")
        .build()
        .unwrap();

    let t = Ticker::new(client, sym).unwrap();
    let pt = t.analyst_price_target().await.unwrap();

    first.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();

    assert_eq!(pt.mean, Some(123.45));
    assert_eq!(pt.high, Some(150.0));
    assert_eq!(pt.low, Some(100.0));
    assert_eq!(pt.number_of_analysts, Some(20));
}
