use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
async fn quote_v7_happy_path() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol":"AAPL",
            "regularMarketPrice": 190.25,
            "regularMarketPreviousClose": 189.50,
            "currency": "USD",
            "fullExchangeName": "NasdaqGS",
            "marketState": "REGULAR"
          }
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::with_quote_base(
        client,
        "AAPL",
        Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap(),
    )
    .unwrap();

    let q = ticker.quote().await.unwrap();
    mock.assert();

    assert_eq!(q.symbol, "AAPL");
    assert_eq!(q.currency.as_deref(), Some("USD"));
    assert_eq!(q.exchange.as_deref(), Some("NasdaqGS"));
    assert_eq!(q.market_state.as_deref(), Some("REGULAR"));
    assert!((q.regular_market_price.unwrap() - 190.25).abs() < 1e-9);
    assert!((q.regular_market_previous_close.unwrap() - 189.50).abs() < 1e-9);
}

#[tokio::test]
async fn fast_info_derives_last_price() {
    let server = MockServer::start();

    // Deliberately omit regularMarketPrice to test fallback → previous close
    let body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol":"MSFT",
            "regularMarketPreviousClose": 421.00,
            "currency": "USD",
            "exchange": "NasdaqGS",
            "marketState": "CLOSED"
          }
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "MSFT");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::with_quote_base(
        client,
        "MSFT",
        Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap(),
    )
    .unwrap();

    let fi = ticker.fast_info().await.unwrap();
    mock.assert();

    assert_eq!(fi.symbol, "MSFT");
    assert!(
        (fi.last_price - 421.00).abs() < 1e-9,
        "fallback to previous close"
    );
    assert_eq!(fi.currency.as_deref(), Some("USD"));
    assert_eq!(fi.exchange.as_deref(), Some("NasdaqGS"));
    assert_eq!(fi.market_state.as_deref(), Some("CLOSED"));
}

#[tokio::test]
#[ignore]
async fn live_quote_smoke() {
    if std::env::var("YF_LIVE").ok().as_deref() != Some("1")
        && std::env::var("YF_RECORD").ok().as_deref() != Some("1")
    {
        return;
    }

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::new(client, "AAPL").unwrap();
    let fi = ticker.fast_info().await.unwrap();

    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        assert!(fi.last_price > 0.0);
        assert_eq!(fi.symbol, "AAPL");
    }
}
