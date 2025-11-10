use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::core::conversions::*;
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

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, "AAPL");

    let q = ticker.quote().await.unwrap();
    mock.assert();

    match q.instrument.id() {
        paft::domain::IdentifierScheme::Security(s) => assert_eq!(s.symbol.as_str(), "AAPL"),
        paft::domain::IdentifierScheme::Prediction(_) => {
            panic!("unexpected instrument identifier scheme")
        }
    }
    assert_eq!(
        q.exchange.as_ref().map(std::string::ToString::to_string),
        Some("NASDAQ".to_string())
    );
    assert_eq!(
        q.market_state
            .as_ref()
            .map(std::string::ToString::to_string),
        Some("REGULAR".to_string())
    );
    assert!((money_to_f64(&q.price.unwrap()) - 190.25).abs() < 1e-9);
    assert!((money_to_f64(&q.previous_close.unwrap()) - 189.50).abs() < 1e-9);
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

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, "MSFT");

    let fi = ticker.fast_info().await.unwrap();
    mock.assert();

    match fi.instrument.id() {
        paft::domain::IdentifierScheme::Security(s) => assert_eq!(s.symbol.as_str(), "MSFT"),
        paft::domain::IdentifierScheme::Prediction(_) => {
            panic!("unexpected instrument identifier scheme")
        }
    }
    assert!(fi.last.is_none());
    assert!(
        (money_to_f64(&fi.previous_close.unwrap()) - 421.00).abs() < 1e-9,
        "fallback to previous close"
    );
    assert_eq!(fi.currency.map(|c| c.to_string()).as_deref(), Some("USD"));
    assert_eq!(
        fi.exchange.map(|e| e.to_string()).as_deref(),
        Some("NASDAQ")
    );
    assert_eq!(
        fi.market_state.map(|s| s.to_string()).as_deref(),
        Some("CLOSED")
    );
}

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_quote_smoke() {
    if std::env::var("YF_LIVE").ok().as_deref() != Some("1")
        && std::env::var("YF_RECORD").ok().as_deref() != Some("1")
    {
        return;
    }

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::new(&client, "AAPL");
    let fi = ticker.fast_info().await.unwrap();

    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        assert!(money_to_f64(&fi.last.unwrap()) > 0.0);
        match fi.instrument.id() {
            paft::domain::IdentifierScheme::Security(s) => assert_eq!(s.symbol.as_str(), "AAPL"),
            paft::domain::IdentifierScheme::Prediction(_) => {
                panic!("unexpected instrument identifier scheme")
            }
        }
    }
}
