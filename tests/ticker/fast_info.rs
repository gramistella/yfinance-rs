use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
async fn fast_info_uses_previous_close_when_price_missing() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [{
          "symbol": "AAPL",
          "regularMarketPrice": null,
          "regularMarketPreviousClose": 199.5,
          "currency": "USD",
          "fullExchangeName": "NasdaqGS"
        }],
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
    let t = Ticker::new(&client, "AAPL");

    let fi = t.fast_info().await.unwrap();
    mock.assert();

    assert_eq!(fi.symbol.as_str(), "AAPL");
    assert!(
        (yfinance_rs::core::conversions::money_to_f64(&fi.previous_close.unwrap()) - 199.5).abs()
            < 1e-9
    );
    assert_eq!(
        fi.exchange.map(|e| e.to_string()).as_deref(),
        Some("NASDAQ")
    );
}
