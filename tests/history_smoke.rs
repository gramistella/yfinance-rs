use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{HistoryBuilder, Range, YfClient};

fn sample_json_ok() -> &'static str {
    // Smallest valid-ish payload with two rows (one complete, one incomplete).
    r#"{
      "chart": {
        "result": [{
          "timestamp": [1700000000, 1700086400],
          "indicators": { "quote": [{
            "open":  [100.0, null],
            "high":  [110.0, 111.0],
            "low":   [ 99.0, 100.0],
            "close": [105.0, 108.0],
            "volume":[1000000, 1200000]
          }]}
        }],
        "error": null
      }
    }"#
}

fn sample_json_empty() -> &'static str {
    r#"{
      "chart": {
        "result": [{
          "timestamp": [],
          "indicators": { "quote": [{ "open": [], "high": [], "low": [], "close": [], "volume": [] }] }
        }],
        "error": null
      }
    }"#
}

#[tokio::test]
async fn history_happy_path() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(sample_json_ok());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = HistoryBuilder::new(&client, "AAPL")
        .range(Range::M6)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    assert_eq!(bars.len(), 1); // second row skipped due to null "open"
    assert_eq!(bars[0].open, 100.0);
    assert_eq!(bars[0].close, 105.0);
}

#[tokio::test]
async fn history_no_data_is_ok() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/MSFT");
        then.status(200).body(sample_json_empty());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = HistoryBuilder::new(&client, "MSFT").fetch().await.unwrap();
    assert!(bars.is_empty());
}

#[tokio::test]
async fn history_absolute_range_happy() {
    use chrono::{Duration, TimeZone, Utc};

    let server = MockServer::start();

    // Define a fixed window.
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let end = start + Duration::days(10);

    // Expect period1/period2, not range.
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/AAPL")
            .query_param("period1", &start.timestamp().to_string())
            .query_param("period2", &end.timestamp().to_string())
            .query_param("interval", "1d")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(sample_json_ok());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = HistoryBuilder::new(&client, "AAPL")
        .between(start, end)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].open, 100.0);
}

#[tokio::test]
async fn history_between_invalid_dates() {
    use chrono::{Duration, TimeZone, Utc};

    let server = MockServer::start();
    // No mock needed; the builder should fail before any HTTP call.

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let start = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
    let end = start - Duration::days(1);

    let err = HistoryBuilder::new(&client, "AAPL")
        .between(start, end)
        .fetch()
        .await
        .unwrap_err();

    assert!(matches!(err, yfinance_rs::YfError::InvalidDates));
}
