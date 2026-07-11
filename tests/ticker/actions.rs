use httpmock::Method::GET;
use httpmock::MockServer;
use std::time::Duration;
use url::Url;
use yfinance_rs::core::{Action, Range, conversions::money_to_f64};
use yfinance_rs::{CacheMode, Ticker, YfClient};

fn date_from_ts(timestamp: i64) -> chrono::NaiveDate {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .unwrap()
        .date_naive()
}

fn body_with_actions() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "timestamp":[1000,2000,3000],
            "indicators":{
              "quote":[{
                "open":[100.0,100.0,100.0],
                "high":[101.0,101.0,101.0],
                "low":[99.0,99.0,99.0],
                "close":[100.0,100.0,100.0],
                "volume":[10,10,10]
              }],
              "adjclose":[{"adjclose":[50.0,100.0,99.0]}]
            },
            "events":{
              "splits":{
                "2000":{"date":2000,"numerator":2,"denominator":1}
              },
              "dividends":{
                "3000":{"date":3000,"amount":1.0}
              }
            }
          }
        ],
        "error":null
      }
    }"#
    .to_string()
}

fn body_with_actions_and_currency() -> String {
    r#"{
      "chart":{
        "result":[
          {
            "meta":{"currency":"USD","instrumentType":"EQUITY"},
            "timestamp":[1000,2000,3000],
            "indicators":{
              "quote":[{
                "open":[100.0,100.0,100.0],
                "high":[101.0,101.0,101.0],
                "low":[99.0,99.0,99.0],
                "close":[100.0,100.0,100.0],
                "volume":[10,10,10]
              }],
              "adjclose":[{"adjclose":[50.0,100.0,99.0]}]
            },
            "events":{
              "splits":{
                "2000":{"date":2000,"numerator":2,"denominator":1}
              },
              "dividends":{
                "3000":{"date":3000,"amount":1.0}
              }
            }
          }
        ],
        "error":null
      }
    }"#
    .to_string()
}

#[tokio::test]
async fn ticker_actions_include_dividends_and_splits() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/TEST")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains")
            .is_true(crate::common::is_daily_max_period_query);
        then.status(200)
            .header("content-type", "application/json")
            .body(body_with_actions());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let t = Ticker::new(&client, "TEST");

    let acts = t.actions(None).await.unwrap();
    mock.assert();

    assert_eq!(acts.len(), 2);
    assert!(acts.iter().any(|action| {
        matches!(
            action,
            Action::Dividend { date, amount }
                if *date == date_from_ts(3000) && (money_to_f64(amount) - 1.0).abs() < 1e-9
        )
    }));
    assert!(acts.iter().any(|action| {
        matches!(
            action,
            Action::Split {
                date,
                numerator,
                denominator,
            } if *date == date_from_ts(2000) && numerator.get() == 2 && denominator.get() == 1
        )
    }));
}

#[tokio::test]
async fn ticker_actions_respect_ticker_cache_bypass() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/TEST")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains")
            .is_true(crate::common::is_daily_max_period_query);
        then.status(200)
            .header("content-type", "application/json")
            .body(body_with_actions_and_currency());
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .cache_ttl(Duration::from_mins(1))
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, "TEST").cache_mode(CacheMode::Bypass);

    ticker.actions(None).await.unwrap();
    ticker.actions(None).await.unwrap();

    mock.assert_calls(2);
}

#[tokio::test]
async fn ticker_actions_skip_invalid_amounts_and_keep_valid_siblings() {
    let server = MockServer::start();

    let body = r#"{
      "chart":{
        "result":[
          {
            "timestamp":[1000],
            "indicators":{
              "quote":[{
                "open":[100.0],
                "high":[101.0],
                "low":[99.0],
                "close":[100.0],
                "volume":[10]
              }]
            },
            "events":{
              "dividends":{
                "2000":{"date":2000,"amount":1e30},
                "3000":{"date":3000,"amount":1.0}
              },
              "capitalGains":{
                "4000":{"date":4000,"amount":1e30},
                "5000":{"date":5000,"amount":2.0}
              }
            }
          }
        ],
        "error":null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v8/finance/chart/TEST")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains")
            .is_true(crate::common::is_daily_max_period_query);
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let t = Ticker::new(&client, "TEST");
    let actions = t.actions(Some(Range::Max)).await.unwrap();
    mock.assert();

    assert_eq!(actions.len(), 2);
    assert!(actions.iter().any(|action| {
        matches!(
            action,
            Action::Dividend { date, amount }
                if *date == date_from_ts(3000) && (money_to_f64(amount) - 1.0).abs() < 1e-9
        )
    }));
    assert!(actions.iter().any(|action| {
        matches!(
            action,
            Action::CapitalGain { date, gain }
                if *date == date_from_ts(5000) && (money_to_f64(gain) - 2.0).abs() < 1e-9
        )
    }));
}
