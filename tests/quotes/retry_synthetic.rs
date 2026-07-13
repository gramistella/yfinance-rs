use crate::common;
use httpmock::Method::GET;
use httpmock::MockServer;
use paft::{Decimal, money::PriceAmount};
use url::Url;

fn usd_price(value: i64) -> PriceAmount {
    PriceAmount::new(Decimal::from(value))
}

#[tokio::test]
async fn batch_quotes_401_then_retry_with_crumb_succeeds() {
    let server = MockServer::start();

    // Respond OK only when the crumb is present (define this first so the
    // second request matches here; the fallback 401 is defined below).
    let ok = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("crumb", "crumb-value");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{
              "quoteResponse": {
                "result": [
                  { "symbol":"AAPL", "quoteType":"EQUITY", "regularMarketPrice": 123.0, "currency":"USD", "fullExchangeName":"NasdaqGS" },
                  { "symbol":"MSFT", "quoteType":"EQUITY", "regularMarketPrice": 456.0, "currency":"USD", "exchange":"NasdaqGS" }
                ],
                "error": null
              }
            }"#);
    });

    // First call returns 401 (no crumb)
    let first = server.mock(|when, then| {
        when.method(GET).path("/v7/finance/quote");
        then.status(401).body("unauthorized");
    });

    // Cookie + crumb endpoints
    let cookie = server.mock(|when, then| {
        when.method(GET).path("/consent");
        then.status(200).header(
            "set-cookie",
            "A=B; Max-Age=315360000; Domain=.yahoo.com; Path=/; Secure; SameSite=None",
        );
    });
    let crumb = server.mock(|when, then| {
        when.method(GET).path("/v1/test/getcrumb");
        then.status(200).body("crumb-value");
    });

    let base = Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap();

    let client = yfinance_rs::YfClient::builder()
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        .base_quote_v7(base)
        .build()
        .unwrap();

    let quotes = yfinance_rs::QuotesBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .fetch()
        .await
        .unwrap();

    // Verify mocks were actually hit
    first.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();

    assert_eq!(quotes.len(), 2);
    let aapl = quotes
        .iter()
        .find(|q| q.instrument.symbol.as_str() == "AAPL")
        .unwrap();
    let msft = quotes
        .iter()
        .find(|q| q.instrument.symbol.as_str() == "MSFT")
        .unwrap();
    assert_eq!(aapl.price, Some(usd_price(123)));
    assert_eq!(msft.price, Some(usd_price(456)));
    assert_eq!(
        aapl.instrument
            .exchange
            .as_ref()
            .map(std::string::ToString::to_string),
        Some("NASDAQ".to_string())
    );
    assert_eq!(
        msft.instrument
            .exchange
            .as_ref()
            .map(std::string::ToString::to_string),
        Some("NASDAQ".to_string())
    );
}

#[tokio::test]
async fn batch_quotes_401_with_stale_cached_crumb_refreshes_before_retry() {
    let server = MockServer::start();

    let bare = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL")
            .is_true(|req| !req.query_params().iter().any(|(k, _)| k == "crumb"));
        then.status(401).body("unauthorized");
    });

    let (cookie, crumb) = common::mock_cookie_crumb(&server);

    let stale = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL")
            .query_param("crumb", "stale-crumb");
        then.status(401).body("unauthorized");
    });

    let ok = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL")
            .query_param("crumb", "crumb-value");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{
              "quoteResponse": {
                "result": [
                  { "symbol":"AAPL", "quoteType":"EQUITY", "regularMarketPrice": 123.0, "currency":"USD", "fullExchangeName":"NasdaqGS" }
                ],
                "error": null
              }
            }"#);
    });

    let client = yfinance_rs::YfClient::builder()
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        ._preauth("cookie", "stale-crumb")
        .build()
        .unwrap();

    let quotes = yfinance_rs::QuotesBuilder::new(&client)
        .symbols(["AAPL"])
        .fetch()
        .await
        .unwrap();

    assert_eq!(
        bare.calls(),
        0,
        "cached OptionalCrumb credentials should be tried before a bare request"
    );
    stale.assert();
    cookie.assert();
    crumb.assert();
    ok.assert();
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].price, Some(usd_price(123)));
}
