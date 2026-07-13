use httpmock::Method::GET;
use httpmock::MockServer;
use paft::Decimal;
use url::Url;
use yfinance_rs::{ProjectionIssue, Ticker, YfClient, YfError, YfWarning};

fn json_decimal(value: &serde_json::Value) -> Decimal {
    value
        .as_number()
        .expect("fixture decimal")
        .to_string()
        .parse()
        .expect("valid fixture decimal")
}

#[tokio::test]
async fn fast_info_uses_previous_close_when_price_missing() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [{
          "symbol": "AAPL",
          "quoteType": "EQUITY",
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

    assert_eq!(fi.snapshot.instrument.symbol.as_str(), "AAPL");
    assert_eq!(
        fi.snapshot.previous_close.unwrap().as_decimal(),
        &Decimal::new(1995, 1)
    );
    assert_eq!(
        fi.snapshot
            .instrument
            .exchange
            .map(|e| e.to_string())
            .as_deref(),
        Some("NASDAQ")
    );
}

#[tokio::test]
async fn fast_info_omits_malformed_optional_moving_average() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [{
          "symbol": "AAPL",
          "quoteType": "EQUITY",
          "regularMarketPrice": 199.5,
          "regularMarketPreviousClose": 198.0,
          "currency": "USD",
          "fiftyDayAverage": "not-a-number",
          "twoHundredDayAverage": 180.0
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
    let ticker = Ticker::new(&client, "AAPL");

    let response = ticker.fast_info_with_diagnostics().await.unwrap();

    assert!(response.data.snapshot.last.is_some());
    assert!(response.data.moving_averages.fifty_day.is_none());
    assert!(response.data.moving_averages.two_hundred_day.is_some());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            endpoint: "fast_info",
            path: "fiftyDayAverage",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "fiftyDayAverage",
                ..
            },
        } if key == "AAPL"
    )));

    let err = Ticker::new(&client, "AAPL")
        .strict()
        .fast_info()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn fast_info_with_diagnostics_errors_without_required_currency() {
    let server = MockServer::start();

    let body = r#"{
      "quoteResponse": {
        "result": [{
          "symbol": "AAPL",
          "quoteType": "EQUITY",
          "regularMarketPrice": 199.5,
          "regularMarketPreviousClose": 198.0,
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
    let ticker = Ticker::new(&client, "AAPL");

    let err = ticker.fast_info_with_diagnostics().await;
    mock.assert();

    assert!(matches!(
        err,
        Err(YfError::InvalidData(message))
            if message.contains("invalid snapshot currency: currency unresolved")
    ));
}

#[tokio::test]
async fn fast_info_maps_snapshot_session_fields_from_v7_quote() {
    let server = MockServer::start();
    let fixture = crate::common::fixture("quote_v7", "AAPL", "json");
    let raw: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let raw_quote = raw["quoteResponse"]["result"]
        .as_array()
        .and_then(|quotes| quotes.first())
        .expect("quote fixture should contain AAPL");

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture);
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, "AAPL");

    let fast_info = ticker.fast_info().await.unwrap();
    mock.assert();

    let snapshot = &fast_info.snapshot;
    assert_eq!(
        snapshot.open.as_ref().unwrap().as_decimal(),
        &json_decimal(&raw_quote["regularMarketOpen"])
    );
    assert_eq!(
        snapshot.day_high.as_ref().unwrap().as_decimal(),
        &json_decimal(&raw_quote["regularMarketDayHigh"])
    );
    assert_eq!(
        snapshot.day_low.as_ref().unwrap().as_decimal(),
        &json_decimal(&raw_quote["regularMarketDayLow"])
    );
    assert_eq!(
        snapshot.volume.as_ref().map(ToString::to_string),
        raw_quote["regularMarketVolume"]
            .as_u64()
            .map(|value| value.to_string())
    );

    assert_eq!(
        fast_info
            .moving_averages
            .fifty_day
            .as_ref()
            .unwrap()
            .amount(),
        json_decimal(&raw_quote["fiftyDayAverage"])
    );
    assert_eq!(
        fast_info
            .moving_averages
            .two_hundred_day
            .as_ref()
            .unwrap()
            .amount(),
        json_decimal(&raw_quote["twoHundredDayAverage"])
    );

    #[cfg(feature = "dataframe")]
    {
        use yfinance_rs::ToDataFrame;

        let df = snapshot.to_dataframe().unwrap();
        assert_eq!(df.height(), 1);
    }
}
