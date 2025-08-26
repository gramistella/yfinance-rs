use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;

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
                  { "symbol":"AAPL", "regularMarketPrice": 123.0, "currency":"USD", "fullExchangeName":"NasdaqGS" },
                  { "symbol":"MSFT", "regularMarketPrice": 456.0, "currency":"USD", "exchange":"NasdaqGS" }
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


    let quotes = yfinance_rs::QuotesBuilder::new(client)
        .unwrap()
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
    let aapl = quotes.iter().find(|q| q.symbol == "AAPL").unwrap();
    let msft = quotes.iter().find(|q| q.symbol == "MSFT").unwrap();
    assert_eq!(aapl.regular_market_price, Some(123.0));
    assert_eq!(msft.regular_market_price, Some(456.0));
    assert_eq!(aapl.currency.as_deref(), Some("USD"));
    assert_eq!(aapl.exchange.as_deref(), Some("NasdaqGS"));
}
