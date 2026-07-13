use httpmock::Method::GET;
use httpmock::{Mock, MockServer};
use paft::Decimal;
use std::num::NonZeroUsize;
use url::Url;

use crate::common;
use yfinance_rs::core::conversions::*;
use yfinance_rs::core::{Interval, Range};
use yfinance_rs::{
    DownloadBuilder, DownloadConcurrency, ProjectionIssue, YfClient, YfError, YfWarning,
};

fn has_more_than_decimals(x: f64, decimals: i32) -> bool {
    if !x.is_finite() {
        return false;
    }
    let scale = 10_f64.powi(decimals);
    let rounded = (x * scale).round();
    (x - rounded / scale).abs() > 1e-12
}

async fn wait_for_mock_calls(
    mock: &Mock<'_>,
    expected: usize,
    timeout: std::time::Duration,
) -> bool {
    let started = tokio::time::Instant::now();
    while started.elapsed() < timeout {
        if mock.calls_async().await >= expected {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    mock.calls_async().await >= expected
}

const fn chart_body_without_instrument_type() -> &'static str {
    r#"{
      "chart": {
        "result": [{
          "meta": {
            "currency": "USD",
            "symbol": "NOKIND",
            "timezone": "America/New_York",
            "gmtoffset": -14400
          },
          "timestamp": [1710000000],
          "indicators": {
            "quote": [{
              "open": [100.0],
              "high": [101.0],
              "low": [99.0],
              "close": [100.5],
              "volume": [1000]
            }],
            "adjclose": [{
              "adjclose": [100.5]
            }]
          }
        }],
        "error": null
      }
    }"#
}

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
        .auto_adjust()
        .prepost(false)
        .actions(true)
        .run()
        .await
        .unwrap();

    m_aapl.assert();
    m_msft.assert();

    let keys: Vec<_> = res
        .entries
        .iter()
        .map(|e| e.instrument.symbol.as_str().to_string())
        .collect();
    assert!(keys.iter().any(|s| s == "AAPL"));
    assert!(keys.iter().any(|s| s == "MSFT"));
}

#[tokio::test]
async fn best_effort_download_keeps_successes_when_symbol_fetch_fails() {
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

    let m_broken = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/BROKEN")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(404);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let response = DownloadBuilder::new(&client)
        .symbols(["AAPL", "BROKEN"])
        .run_with_diagnostics()
        .await
        .unwrap();

    m_aapl.assert();
    m_broken.assert();

    assert_eq!(response.data.entries.len(), 1);
    assert_eq!(response.data.entries[0].instrument.symbol.as_str(), "AAPL");
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::DroppedItem {
            endpoint: "download",
            item: "download_entry",
            key: Some(key),
            reason: ProjectionIssue::ProviderError { message },
        } if key == "BROKEN"
            && message.contains("history fetch failed: Not found at")
            && message.contains("/v8/finance/chart/BROKEN")
    )));
}

#[tokio::test]
async fn best_effort_download_uses_untyped_instrument_without_instrument_metadata() {
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

    let m_missing = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/NOKIND")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(chart_body_without_instrument_type());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let response = DownloadBuilder::new(&client)
        .symbols(["AAPL", "NOKIND"])
        .run_with_diagnostics()
        .await
        .unwrap();

    m_aapl.assert();
    m_missing.assert();

    assert_eq!(response.data.entries.len(), 2);
    assert!(
        response
            .data
            .entries
            .iter()
            .any(|entry| entry.instrument.symbol.as_str() == "AAPL")
    );
    let fallback = response
        .data
        .entries
        .iter()
        .find(|entry| entry.instrument.symbol.as_str() == "NOKIND")
        .expect("NOKIND should use untyped fallback instrument");
    let fallback_kind: &str = fallback.instrument.kind.code();
    assert_eq!(fallback_kind, "YAHOO_DOWNLOAD_UNTYPED");
    assert_eq!(fallback.history.candles.len(), 1);
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::RepairedData {
            endpoint: "download",
            item: "download_entry",
            key: Some(key),
            repair: "used untyped Yahoo download instrument because chart.meta.instrumentType was missing",
        } if key == "NOKIND"
    )));
}

#[tokio::test]
async fn strict_download_errors_without_instrument_metadata() {
    let server = common::setup_server();

    let m_missing = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/NOKIND")
            .query_param("range", "6mo")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(chart_body_without_instrument_type());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let err = DownloadBuilder::new(&client)
        .symbols(["NOKIND"])
        .strict()
        .run()
        .await
        .unwrap_err();

    m_missing.assert();

    let YfError::DataQuality(warning) = err else {
        panic!("expected data-quality error");
    };
    assert!(matches!(
        &*warning,
        YfWarning::RepairedData {
            endpoint: "download",
            item: "download_entry",
            key: Some(key),
            repair: "used untyped Yahoo download instrument because chart.meta.instrumentType was missing",
        } if key == "NOKIND"
    ));
}

#[tokio::test]
async fn strict_download_uses_per_response_instruments_after_side_cache_eviction() {
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
        .side_cache_max_entries(NonZeroUsize::new(1).unwrap())
        .build()
        .unwrap();

    let response = DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .concurrency(DownloadConcurrency::new(1).unwrap())
        .strict()
        .run_with_diagnostics()
        .await
        .unwrap();

    m_aapl.assert();
    m_msft.assert();

    let symbols: Vec<_> = response
        .data
        .entries
        .iter()
        .map(|entry| entry.instrument.symbol.as_str())
        .collect();
    assert_eq!(symbols, vec!["AAPL", "MSFT"]);
    assert!(
        response
            .data
            .entries
            .iter()
            .all(|entry| entry.instrument.kind.code() != "YAHOO_DOWNLOAD_UNTYPED")
    );
}

#[tokio::test]
async fn download_respects_configured_concurrency_limit() {
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
            .delay(std::time::Duration::from_secs(1))
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

    let download = DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .concurrency(DownloadConcurrency::new(1).unwrap());
    let handle = tokio::spawn(async move { download.run().await });

    assert!(
        wait_for_mock_calls(&m_aapl, 1, std::time::Duration::from_millis(750)).await,
        "first symbol was not requested"
    );
    assert_eq!(
        m_msft.calls_async().await,
        0,
        "second symbol started before the configured concurrency slot was released"
    );

    let res = handle.await.unwrap().unwrap();

    m_aapl.assert();
    m_msft.assert();

    let symbols: Vec<_> = res
        .entries
        .iter()
        .map(|entry| entry.instrument.symbol.as_str())
        .collect();
    assert_eq!(symbols, vec!["AAPL", "MSFT"]);
}

#[test]
fn download_concurrency_rejects_zero() {
    match DownloadConcurrency::new(0) {
        Err(YfError::InvalidParams(message)) => {
            assert!(message.contains("concurrency"));
        }
        Err(other) => panic!("expected invalid params error, got {other:?}"),
        Ok(_) => panic!("expected zero concurrency to be rejected"),
    }
}

#[tokio::test]
async fn download_rejects_invalid_symbol_before_request() {
    let symbol = "A".repeat(65);
    let client = YfClient::default();

    let result = DownloadBuilder::new(&client).symbols([symbol]).run().await;

    match result {
        Err(YfError::InvalidParams(message)) => {
            assert!(message.contains("invalid symbol"));
        }
        Err(other) => panic!("expected invalid symbol error, got {other:?}"),
        Ok(_) => panic!("expected invalid symbol error"),
    }
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
        .auto_adjust()
        .prepost(false)
        .actions(true)
        .run()
        .await
        .unwrap();

    q1.assert();
    q2.assert();

    assert_eq!(res.entries.len(), 2);
    let aapl = res
        .entries
        .iter()
        .find(|e| e.instrument.symbol.as_str() == "AAPL")
        .unwrap();
    let msft = res
        .entries
        .iter()
        .find(|e| e.instrument.symbol.as_str() == "MSFT")
        .unwrap();
    assert!(!aapl.history.candles.is_empty());
    assert!(!msft.history.candles.is_empty());
}

#[tokio::test]
async fn download_between_rejects_invalid_dates_as_top_level_error() {
    use chrono::{TimeZone, Utc};

    let start = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let client = YfClient::default();

    let err = DownloadBuilder::new(&client)
        .symbols(["AAPL", "MSFT"])
        .between(start, end)
        .run()
        .await
        .unwrap_err();

    assert!(matches!(err, YfError::InvalidDates));
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
        .auto_adjust()
        .run()
        .await
        .unwrap();

    let back = DownloadBuilder::new(&client2)
        .symbols(["AAPL"])
        .back_adjust() // uses an internal adjusted fetch for OHL
        .run()
        .await
        .unwrap();

    m1_aapl.assert(); // exactly 1
    m2_aapl.assert(); // exactly 1

    let adj_history = &adj
        .entries
        .iter()
        .find(|e| e.instrument.symbol.as_str() == "AAPL")
        .unwrap()
        .history;
    let back_history = &back
        .entries
        .iter()
        .find(|e| e.instrument.symbol.as_str() == "AAPL")
        .unwrap()
        .history;
    let adjusted = yfinance_rs::PriceBasis::provider_latest_adjusted();
    assert_eq!(
        adj_history.price_basis,
        yfinance_rs::OhlcPriceBasis::uniform(adjusted)
    );
    assert_eq!(
        back_history.price_basis,
        yfinance_rs::OhlcPriceBasis::per_field(
            adjusted,
            adjusted,
            adjusted,
            yfinance_rs::PriceBasis::raw()
        )
    );

    let a = &adj_history.candles;
    let b = &back_history.candles;

    assert_eq!(a.len(), b.len(), "same number of bars");
    for (ca, cb) in a.iter().zip(b.iter()) {
        assert!((money_to_f64(&ca.ohlc.open) - money_to_f64(&cb.ohlc.open)).abs() < 1e-9);
        assert!((money_to_f64(&ca.ohlc.high) - money_to_f64(&cb.ohlc.high)).abs() < 1e-9);
        assert!((money_to_f64(&ca.ohlc.low) - money_to_f64(&cb.ohlc.low)).abs() < 1e-9);
        // close may differ due to back_adjust
    }
    assert!(!a.is_empty(), "expected some data");
}

#[tokio::test]
async fn rounding_uses_recorded_price_hint() {
    use yfinance_rs::core::conversions::money_to_f64;

    let server = MockServer::start();

    let m_aapl = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/AAPL")
            .query_param_exists("interval")
            .query_param_exists("includePrePost")
            .query_param_exists("range");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("history_chart", "AAPL", "json"));
    });

    let m_msft = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/MSFT")
            .query_param_exists("interval")
            .query_param_exists("includePrePost")
            .query_param_exists("range");
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
        .run()
        .await
        .unwrap();

    m_aapl.assert();
    m_msft.assert();

    for entry in &res.entries {
        for c in &entry.history.candles {
            assert!(!has_more_than_decimals(money_to_f64(&c.ohlc.open), 2));
            assert!(!has_more_than_decimals(money_to_f64(&c.ohlc.high), 2));
            assert!(!has_more_than_decimals(money_to_f64(&c.ohlc.low), 2));
            assert!(!has_more_than_decimals(money_to_f64(&c.ohlc.close), 2));
            if let Some(close_unadj) = c.close_unadj.as_ref() {
                assert!(!has_more_than_decimals(money_to_f64(close_unadj), 2));
            }
        }
    }
}

#[tokio::test]
async fn rounding_uses_recorded_minor_unit_price_hint_before_scaling() {
    let fixture = common::fixture("history_chart", "TSCO.L", "json");
    let raw: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let result = &raw["chart"]["result"][0];
    let meta = &result["meta"];
    assert_eq!(meta["currency"].as_str(), Some("GBp"));
    let price_hint = u32::try_from(meta["priceHint"].as_u64().unwrap()).unwrap();
    let quote = &result["indicators"]["quote"][0];
    let expected_open = rounded_minor_unit_field(quote, "open", price_hint);
    let scale_first_open = scale_first_rounded_minor_unit_field(quote, "open", price_hint);
    assert_ne!(
        expected_open, scale_first_open,
        "fixture should expose the previous scale-first rounding bug"
    );

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/TSCO.L")
            .query_param("range", "5d")
            .query_param("interval", "1d")
            .query_param("includePrePost", "false")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let response = DownloadBuilder::new(&client)
        .symbols(["TSCO.L"])
        .range(Range::D5)
        .interval(Interval::D1)
        .rounding(true)
        .run()
        .await
        .unwrap();

    mock.assert();

    let candle = &response.entries[0].history.candles[0];
    assert_eq!(candle.currency.to_string(), "GBP");
    assert_eq!(candle.ohlc.open.as_decimal(), &expected_open);
    assert_eq!(
        candle.ohlc.high.as_decimal(),
        &rounded_minor_unit_field(quote, "high", price_hint)
    );
    assert_eq!(
        candle.ohlc.low.as_decimal(),
        &rounded_minor_unit_field(quote, "low", price_hint)
    );
    assert_eq!(
        candle.ohlc.close.as_decimal(),
        &rounded_minor_unit_field(quote, "close", price_hint)
    );
    assert_ne!(candle.ohlc.open.as_decimal(), &scale_first_open);
}

fn rounded_minor_unit_field(quote: &serde_json::Value, field: &str, price_hint: u32) -> Decimal {
    provider_decimal_field(quote, field)
        .round_dp(price_hint)
        .checked_mul(Decimal::new(1, 2))
        .unwrap()
}

fn scale_first_rounded_minor_unit_field(
    quote: &serde_json::Value,
    field: &str,
    price_hint: u32,
) -> Decimal {
    provider_decimal_field(quote, field)
        .checked_mul(Decimal::new(1, 2))
        .unwrap()
        .round_dp(price_hint)
}

fn provider_decimal_field(quote: &serde_json::Value, field: &str) -> Decimal {
    quote[field][0]
        .as_number()
        .expect("fixture field should be numeric")
        .to_string()
        .parse()
        .expect("fixture field should fit Decimal")
}
