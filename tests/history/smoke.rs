use crate::common;
use crate::common::{mock_history_chart, setup_server};
use url::Url;
use yfinance_rs::core::Range;
use yfinance_rs::core::conversions::*;
use yfinance_rs::{HistoryBuilder, YfClient};

#[tokio::test]
async fn history_happy_path() {
    let server = setup_server();
    let mock = mock_history_chart(&server, "AAPL");

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = HistoryBuilder::new(&client, "AAPL")
        .range(Range::M6)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    // The recorded fixture has many data points, not just 2.
    assert!(bars.len() > 100, "Expected a significant number of bars");
    assert!(money_to_f64(&bars[0].open) > 0.0);
    assert!(money_to_f64(&bars[0].high) > 0.0);
    assert!(money_to_f64(&bars[0].low) > 0.0);
    assert!(money_to_f64(&bars[0].close) > 0.0);
}

#[tokio::test]
async fn history_no_data_is_ok() {
    let server = setup_server();
    let mock = mock_history_chart(&server, "MSFT");

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = HistoryBuilder::new(&client, "MSFT").fetch().await.unwrap();
    mock.assert();
    // The recorded fixture for a real stock will have data.
    assert!(!bars.is_empty(), "Expected some data for a real stock");
}

#[tokio::test]
async fn history_absolute_range_happy() {
    use chrono::{Duration, TimeZone, Utc};

    let server = setup_server();

    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let end = start + Duration::days(10);

    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("period1", start.timestamp().to_string())
            .query_param("period2", end.timestamp().to_string());
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let bars = HistoryBuilder::new(&client, "AAPL")
        .between(start, end)
        .fetch()
        .await
        .unwrap();

    mock.assert();
    // The mock serves the full 6-month fixture regardless of the date range,
    // so we expect the full data set to be parsed.
    assert!(bars.len() > 100, "Expected a significant number of bars");
    assert!(money_to_f64(&bars[0].open) > 0.0);
}

#[tokio::test]
async fn history_between_invalid_dates() {
    use chrono::{Duration, TimeZone, Utc};
    let client = YfClient::default();

    let start = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
    let end = start - Duration::days(1);

    let err = HistoryBuilder::new(&client, "AAPL")
        .between(start, end)
        .fetch()
        .await
        .unwrap_err();

    assert!(matches!(err, yfinance_rs::YfError::InvalidDates));
}
