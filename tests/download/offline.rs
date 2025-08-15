use httpmock::Method::GET;
use url::Url;

use crate::common;
use yfinance_rs::{DownloadBuilder, Interval, Range, YfClient};

#[tokio::test]
async fn download_multi_symbols_happy_path() {
    let server = common::setup_server();

    let m_aapl = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let m_msft = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/MSFT")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "MSFT", "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let res = DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .range(Range::M6)
        .interval(Interval::D1)
        .auto_adjust(true)
        .prepost(false)
        .actions(true)
        .run()
        .await
        .unwrap();

    m_aapl.assert();
    m_msft.assert();

    assert!(res.series.get("AAPL").is_some());
    assert!(res.series.get("MSFT").is_some());
    assert!(res.meta.contains_key("AAPL"));
    assert!(res.meta.contains_key("MSFT"));
    assert!(res.actions.contains_key("AAPL"));
    assert!(res.actions.contains_key("MSFT"));
}

#[tokio::test]
async fn download_between_params_applied_to_all_symbols() {
    use chrono::{TimeZone, Utc};
    let server = httpmock::MockServer::start();

    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();

    let q1 = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("period1", &start.timestamp().to_string())
            .query_param("period2", &end.timestamp().to_string())
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let q2 = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/MSFT")
            .query_param("period1", &start.timestamp().to_string())
            .query_param("period2", &end.timestamp().to_string())
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "MSFT", "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let res = DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .between(start, end)
        .interval(Interval::D1)
        .auto_adjust(true)
        .prepost(false)
        .actions(true)
        .run()
        .await
        .unwrap();

    q1.assert();
    q2.assert();

    assert_eq!(res.series.len(), 2);
    assert!(!res.series["AAPL"].is_empty());
    assert!(!res.series["MSFT"].is_empty());
}

#[tokio::test]
async fn download_requires_symbols() {
    let client = YfClient::builder().build().unwrap();

    let err = DownloadBuilder::new(&client).run().await.unwrap_err();
    match err {
        yfinance_rs::YfError::Data(s) => assert!(s.contains("no symbols")),
        _ => panic!("expected Data error for no symbols"),
    }
}
