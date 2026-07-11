use httpmock::Method::GET;
use httpmock::MockServer;
use std::time::Duration;
use url::Url;
use yfinance_rs::core::Range;
use yfinance_rs::core::conversions::money_to_f64;
use yfinance_rs::{HistoryBuilder, ProjectionIssue, Ticker, YfClient, YfError, YfWarning};

fn meta_body() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "meta": {
              "timezone":"EDT",
              "exchangeTimezoneName":"America/New_York",
              "gmtoffset": -14400
            },
            "timestamp": [],
            "indicators": {
              "quote":[{ "open":[], "high":[], "low":[], "close":[], "volume":[] }],
              "adjclose":[{ "adjclose":[] }]
            }
          }
        ],
        "error": null
      }
    }"#
    .to_string()
}

fn missing_timestamp_empty_series_body() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "meta": {
              "timezone":"EDT",
              "exchangeTimezoneName":"America/New_York",
              "gmtoffset": -14400
            },
            "indicators": {
              "quote":[{}],
              "adjclose":[{}]
            }
          }
        ],
        "error": null
      }
    }"#
    .to_string()
}

fn missing_timestamp_with_quote_data_body() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "meta": {
              "timezone":"EDT",
              "exchangeTimezoneName":"America/New_York",
              "gmtoffset": -14400
            },
            "indicators": {
              "quote":[{
                "open":[100.0],
                "high":[101.0],
                "low":[99.0],
                "close":[100.5],
                "volume":[1000]
              }],
              "adjclose":[{ "adjclose":[100.5] }]
            }
          }
        ],
        "error": null
      }
    }"#
    .to_string()
}

fn malformed_timezone_body() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "meta": { "symbol": "MSFT", "timezone":"Not/A_Timezone", "gmtoffset": -14400 },
            "timestamp": [],
            "indicators": {
              "quote":[{ "open":[], "high":[], "low":[], "close":[], "volume":[] }],
              "adjclose":[{ "adjclose":[] }]
            }
          }
        ],
        "error": null
      }
    }"#
    .to_string()
}

fn malformed_instrument_type_body() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "meta": {
              "symbol": "BADTYPE",
              "instrumentType": "!!!",
              "currency": "USD",
              "timezone":"EDT",
              "exchangeTimezoneName":"America/New_York",
              "gmtoffset": -14400
            },
            "timestamp": [1704067200],
            "indicators": {
              "quote":[{
                "open":[100.0],
                "high":[101.0],
                "low":[99.0],
                "close":[100.5],
                "volume":[1000]
              }],
              "adjclose":[{ "adjclose":[100.5] }]
            }
          }
        ],
        "error": null
      }
    }"#
    .to_string()
}

fn mismatched_granularity_body() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "meta": { "dataGranularity": "3mo" },
            "timestamp": [],
            "indicators": {
              "quote":[{ "open":[], "high":[], "low":[], "close":[], "volume":[] }],
              "adjclose":[{ "adjclose":[] }]
            }
          }
        ],
        "error": null
      }
    }"#
    .to_string()
}

#[tokio::test]
async fn get_history_metadata_returns_timezone() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/MSFT")
            .query_param("range", "1d")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(meta_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let t = Ticker::new(&client, "MSFT");
    let meta = t.get_history_metadata(Some(Range::D1)).await.unwrap();

    mock.assert();
    let m = meta.expect("meta should be Some");
    assert_eq!(
        m.timezone.as_ref().map(std::string::ToString::to_string),
        Some("America/New_York".to_string())
    );
    assert_eq!(m.utc_offset_seconds, Some(-14400));
}

#[tokio::test]
async fn missing_timestamp_with_empty_chart_series_returns_empty_history() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "1d")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(missing_timestamp_empty_series_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let response = HistoryBuilder::new(&client, "AAPL")
        .range(Range::D1)
        .fetch_full_with_diagnostics()
        .await
        .unwrap();

    mock.assert();
    assert!(response.diagnostics.is_empty());
    assert!(response.data.candles.is_empty());
    assert!(response.data.actions.is_empty());
    assert_eq!(
        response
            .data
            .meta
            .as_ref()
            .and_then(|meta| meta.timezone.as_ref())
            .map(std::string::ToString::to_string),
        Some("America/New_York".to_string())
    );
}

#[tokio::test]
async fn mismatched_chart_granularity_is_rejected_in_every_quality_mode() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "1d")
            .query_param("interval", "1d");
        then.status(200)
            .header("content-type", "application/json")
            .body(mismatched_granularity_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .cache_ttl(Duration::from_mins(1))
        .build()
        .unwrap();

    for strict in [false, true] {
        let builder = HistoryBuilder::new(&client, "AAPL").range(Range::D1);
        let err = if strict {
            builder.strict().fetch_full().await.unwrap_err()
        } else {
            builder.fetch_full().await.unwrap_err()
        };

        assert!(matches!(
            err,
            YfError::InvalidData(message)
                if message.contains("3mo") && message.contains("1d")
        ));
    }

    mock.assert_calls(2);
}

#[tokio::test]
async fn missing_timestamp_with_quote_data_remains_malformed() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "1d")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(missing_timestamp_with_quote_data_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let err = HistoryBuilder::new(&client, "AAPL")
        .range(Range::D1)
        .fetch_full()
        .await
        .unwrap_err();

    mock.assert();
    assert!(matches!(err, YfError::MissingData(message) if message == "missing timestamps"));
}

#[tokio::test]
async fn malformed_history_instrument_type_does_not_abort_candles() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/BADTYPE")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(malformed_instrument_type_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let response = HistoryBuilder::new(&client, "BADTYPE")
        .fetch_full_with_diagnostics()
        .await
        .unwrap();

    mock.assert();
    assert!(response.diagnostics.is_empty());
    assert_eq!(response.data.candles.len(), 1);
    assert!((money_to_f64(&response.data.candles[0].ohlc.close) - 100.5).abs() < 1e-9);
}

#[tokio::test]
async fn history_metadata_uses_exchange_timezone_name_without_warning() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/MSFT")
            .query_param("range", "1d")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(meta_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let response = Ticker::new(&client, "MSFT")
        .history_builder()
        .range(Range::D1)
        .fetch_full_with_diagnostics()
        .await
        .unwrap();

    mock.assert();
    assert!(response.diagnostics.is_empty());
    assert_eq!(
        response
            .data
            .meta
            .as_ref()
            .and_then(|meta| meta.timezone.as_ref())
            .map(std::string::ToString::to_string),
        Some("America/New_York".to_string())
    );
}

#[tokio::test]
async fn malformed_history_timezone_is_reported_as_projection_loss() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/MSFT")
            .query_param("range", "1d")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(malformed_timezone_body());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let response = Ticker::new(&client, "MSFT")
        .history_builder()
        .range(Range::D1)
        .fetch_full_with_diagnostics()
        .await
        .unwrap();

    assert!(response.data.meta.is_some());
    assert!(
        response
            .data
            .meta
            .as_ref()
            .is_some_and(|meta| meta.timezone.is_none())
    );
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "chart.meta.timezone",
            reason: ProjectionIssue::InvalidField {
                field: "timezone",
                ..
            },
            ..
        }
    )));

    let err = Ticker::new(&client, "MSFT")
        .history_builder()
        .strict()
        .range(Range::D1)
        .fetch_full()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}
