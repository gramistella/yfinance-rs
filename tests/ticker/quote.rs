use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::core::conversions::*;
use yfinance_rs::{ProjectionIssue, Ticker, YfClient, YfError, YfWarning};

#[tokio::test]
async fn quote_v7_happy_path() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol":"AAPL",
            "quoteType": "EQUITY",
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

    assert_eq!(q.instrument.symbol.as_str(), "AAPL");
    assert_eq!(
        q.instrument
            .exchange
            .as_ref()
            .map(std::string::ToString::to_string),
        Some("NASDAQ".to_string())
    );
    assert_eq!(
        q.market_state
            .as_ref()
            .map(std::string::ToString::to_string),
        Some("REGULAR".to_string())
    );
    assert_eq!(q.price.unwrap().as_decimal().to_string(), "190.25");
    assert_eq!(q.previous_close.unwrap().as_decimal().to_string(), "189.50");
}

#[tokio::test]
async fn quote_with_diagnostics_errors_on_invalid_required_currency() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol":"AAPL",
            "quoteType": "EQUITY",
            "regularMarketPrice": 190.25,
            "regularMarketPreviousClose": 189.50,
            "currency": "!!!"
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

    let err = ticker.quote_with_diagnostics().await;
    mock.assert();

    assert!(matches!(
        err,
        Err(YfError::InvalidData(message))
            if message.contains("invalid quote currency: invalid currency code !!!")
    ));
}

#[tokio::test]
async fn quote_with_diagnostics_reports_invalid_optional_provider_fields() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol":"AAPL",
            "quoteType": "EQUITY",
            "regularMarketPrice": 190.25,
            "currency": "USD",
            "fullExchangeName": "!!!",
            "marketState": "!!!",
            "regularMarketTime": 9223372036854775807
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

    let response = ticker.quote_with_diagnostics().await.unwrap();
    mock.assert();

    assert!(response.data.instrument.exchange.is_none());
    assert!(response.data.market_state.is_none());
    assert!(response.data.as_of.is_none());
    for path in ["fullExchangeName", "marketState", "regularMarketTime"] {
        assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
            warning,
            YfWarning::OmittedPresentField {
                endpoint: "quote",
                path: warning_path,
                key: Some(key),
                reason: ProjectionIssue::InvalidField { .. },
            } if *warning_path == path && key == "AAPL"
        )));
    }
}

#[tokio::test]
async fn quote_with_diagnostics_omits_malformed_optional_price() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol":"AAPL",
            "quoteType": "EQUITY",
            "regularMarketPrice": "not-a-number",
            "regularMarketPreviousClose": 189.50,
            "currency": "USD"
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

    let response = ticker.quote_with_diagnostics().await.unwrap();
    mock.assert();

    assert_eq!(response.data.instrument.symbol.as_str(), "AAPL");
    assert!(response.data.price.is_none());
    assert!(response.data.previous_close.is_some());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            endpoint: "quote",
            path: "regularMarketPrice",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "regularMarketPrice",
                ..
            },
        } if key == "AAPL"
    )));
}

#[tokio::test]
async fn quote_v7_usd_crypto_price_keeps_provider_precision() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol":"XRPUSD",
            "regularMarketPrice": 0.612345,
            "regularMarketPreviousClose": 0.6000010000000000000000000001,
            "currency": "USD",
            "quoteType": "CRYPTOCURRENCY"
          }
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "XRPUSD");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, "XRPUSD");

    let q = ticker.quote().await.unwrap();
    mock.assert();

    let price = q.price.as_ref().unwrap();
    assert_eq!(price.as_decimal().to_string(), "0.612345");
    assert_eq!(
        q.previous_close.as_ref().unwrap().as_decimal().to_string(),
        "0.6000010000000000000000000001"
    );
    assert_eq!(q.currency.to_string(), "USD");
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
            "quoteType": "EQUITY",
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

    assert_eq!(fi.snapshot.instrument.symbol.as_str(), "MSFT");
    assert!(fi.snapshot.last.is_none());
    let previous_close = fi.snapshot.previous_close.as_ref().unwrap();
    assert!(
        (money_to_f64(previous_close) - 421.00).abs() < 1e-9,
        "fallback to previous close"
    );
    assert_eq!(fi.snapshot.currency.to_string(), "USD");
    assert_eq!(
        fi.snapshot
            .instrument
            .exchange
            .map(|e| e.to_string())
            .as_deref(),
        Some("NASDAQ")
    );
    assert_eq!(
        fi.snapshot.market_state.map(|s| s.to_string()).as_deref(),
        Some("CLOSED")
    );
}

#[tokio::test]
async fn quote_v7_missing_quote_type_returns_error() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [
          {
            "symbol":"AAPL",
            "regularMarketPrice": 190.25,
            "currency": "USD"
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

    let err = ticker.quote().await.unwrap_err();
    mock.assert();

    assert!(matches!(err, YfError::MissingData(message) if message.contains("quoteType")));
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
        assert!(money_to_f64(&fi.snapshot.last.unwrap()) > 0.0);
        assert_eq!(fi.snapshot.instrument.symbol.as_str(), "AAPL");
    }
}
