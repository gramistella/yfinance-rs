use httpmock::Method::GET;
use url::Url;

use crate::common;
use yfinance_rs::core::conversions::*;
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
            .query_param("events", "div|split|capitalGains");
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
            .query_param("events", "div|split|capitalGains");
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

    assert!(res.series.contains_key("AAPL"));
    assert!(res.series.contains_key("MSFT"));
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
            .query_param("period1", start.timestamp().to_string())
            .query_param("period2", end.timestamp().to_string())
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let q2 = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/MSFT")
            .query_param("period1", start.timestamp().to_string())
            .query_param("period2", end.timestamp().to_string())
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
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

/* ---------- Parity knob checks using cached live fixtures ---------- */

#[tokio::test]
async fn download_back_adjust_offline() {
    // Run adjusted and back-adjusted on different mock servers so each mock sees 1 hit.
    let server1 = common::setup_server();
    let server2 = common::setup_server();

    let m1_aapl = server1.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let m2_aapl = server2.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let client1 = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server1.base_url())).unwrap())
        .build()
        .unwrap();

    let client2 = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server2.base_url())).unwrap())
        .build()
        .unwrap();

    let adj = DownloadBuilder::new(&client1)
        .symbols(["AAPL"])
        .auto_adjust(true)
        .back_adjust(false)
        .run()
        .await
        .unwrap();

    let back = DownloadBuilder::new(&client2)
        .symbols(["AAPL"])
        .auto_adjust(false) // ignored internally when back_adjust(true)
        .back_adjust(true)
        .run()
        .await
        .unwrap();

    m1_aapl.assert(); // exactly 1
    m2_aapl.assert(); // exactly 1

    let a = adj.series.get("AAPL").unwrap();
    let b = back.series.get("AAPL").unwrap();

    assert_eq!(a.len(), b.len(), "same number of bars");
    for (ca, cb) in a.iter().zip(b.iter()) {
        assert!((money_to_f64(&ca.open) - money_to_f64(&cb.open)).abs() < 1e-9);
        assert!((money_to_f64(&ca.high) - money_to_f64(&cb.high)).abs() < 1e-9);
        assert!((money_to_f64(&ca.low) - money_to_f64(&cb.low)).abs() < 1e-9);
        // close may differ due to back_adjust
    }
    assert!(!a.is_empty(), "expected some data");
}

#[tokio::test]
async fn download_repair_is_noop_on_clean_data_offline() {
    // Run base and repair=true on different mock servers so each mock sees 1 hit.
    let server1 = common::setup_server();
    let server2 = common::setup_server();

    let m1_aapl = server1.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let m2_aapl = server2.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(common::fixture("history_chart", "AAPL", "json"));
    });

    let client1 = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server1.base_url())).unwrap())
        .build()
        .unwrap();

    let client2 = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server2.base_url())).unwrap())
        .build()
        .unwrap();

    let base_run = DownloadBuilder::new(&client1)
        .symbols(["AAPL"])
        .run()
        .await
        .unwrap();

    let repair_run = DownloadBuilder::new(&client2)
        .symbols(["AAPL"])
        .repair(true)
        .run()
        .await
        .unwrap();

    m1_aapl.assert(); // exactly 1
    m2_aapl.assert(); // exactly 1

    let a = base_run.series.get("AAPL").unwrap();
    let b = repair_run.series.get("AAPL").unwrap();

    assert_eq!(a.len(), b.len());
    for (ca, cb) in a.iter().zip(b.iter()) {
        assert!((money_to_f64(&ca.open) - money_to_f64(&cb.open)).abs() < 1e-12);
        assert!((money_to_f64(&ca.high) - money_to_f64(&cb.high)).abs() < 1e-12);
        assert!((money_to_f64(&ca.low) - money_to_f64(&cb.low)).abs() < 1e-12);
        assert!((money_to_f64(&ca.close) - money_to_f64(&cb.close)).abs() < 1e-12);
    }
}

#[tokio::test]
async fn download_rounding_and_keepna_offline() {
    let server = crate::common::setup_server();

    let m_aapl = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v8/finance/chart/AAPL")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("history_chart", "AAPL", "json"));
    });

    let m_msft = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/v8/finance/chart/MSFT")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("history_chart", "MSFT", "json"));
    });

    let client = yfinance_rs::YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let res = yfinance_rs::DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .rounding(true)
        .keepna(true)
        .run()
        .await
        .unwrap();

    m_aapl.assert();
    m_msft.assert();

    fn has_more_than_two_decimals(x: f64) -> bool {
        if !x.is_finite() {
            return false;
        }
        let cents = (x * 100.0).round();
        (x - cents / 100.0).abs() > 1e-12
    }

    for bars in res.series.values() {
        for c in bars {
            assert!(!has_more_than_two_decimals(money_to_f64(&c.open)));
            assert!(!has_more_than_two_decimals(money_to_f64(&c.high)));
            assert!(!has_more_than_two_decimals(money_to_f64(&c.low)));
            assert!(!has_more_than_two_decimals(money_to_f64(&c.close)));
        }
    }
}
