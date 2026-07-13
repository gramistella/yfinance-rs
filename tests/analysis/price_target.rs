use httpmock::Method::GET;
use httpmock::MockServer;
use paft::Decimal;
use paft::money::{Currency, IsoCurrency, Price};
use url::Url;
use yfinance_rs::{Ticker, YfClient};

fn usd_price(value: &str) -> Price {
    Price::new(
        value.parse::<Decimal>().expect("known-good decimal price"),
        Currency::Iso(IsoCurrency::USD),
    )
}

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

#[tokio::test]
async fn offline_price_target_preserves_exact_decimals() {
    let server = MockServer::start();
    let sym = "AAPL";

    let body = r#"{
      "quoteSummary": {
        "result": [{
          "financialData": {
            "targetMeanPrice": { "raw": 1234567890.123456789012345678 },
            "targetHighPrice": { "raw": 250.031600002 },
            "targetLowPrice":  { "raw": 150.0 },
            "numberOfAnalystOpinions": { "raw": 31 }
          }
        }],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "financialData")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("quote_v7", sym));
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(&client, sym);
    let pt = t.analyst_price_target(None).await.unwrap();

    mock.assert();
    quote_mock.assert();

    assert_eq!(pt.mean, Some(usd_price("1234567890.123456789012345678")));
    assert_eq!(pt.high, Some(usd_price("250.031600002")));
    assert_eq!(pt.low, Some(usd_price("150.0")));
    assert_eq!(pt.number_of_analysts, Some(31));
}

#[tokio::test]
async fn price_target_invalid_crumb_then_retry_succeeds() {
    let server = MockServer::start();
    let sym = "MSFT";

    let first = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
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
            .path(format!("/v10/finance/quoteSummary/{sym}"))
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
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("quote_v7", sym));
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        ._preauth("cookie", "stale")
        .build()
        .unwrap();

    let t = Ticker::new(&client, sym);
    let pt = t.analyst_price_target(None).await.unwrap();

    first.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();
    quote_mock.assert();

    assert_eq!(pt.mean, Some(usd_price("123.45")));
    assert_eq!(pt.high, Some(usd_price("150.0")));
    assert_eq!(pt.low, Some(usd_price("100.0")));
    assert_eq!(pt.number_of_analysts, Some(20));
}
