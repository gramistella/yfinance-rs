use httpmock::{Method::GET, MockServer};
use paft::Decimal;
use std::str::FromStr;
use url::Url;
use yfinance_rs::{Action, HistoryBuilder, YfClient};

const BODY: &str = r#"{
  "chart":{"result":[{
    "meta":{"currency":"USD","symbol":"TEST","instrumentType":"EQUITY","priceHint":2},
    "timestamp":[1704067200],
    "indicators":{
      "quote":[{
        "open":[0.13002200424671173],
        "high":[0.13895100355148315],
        "low":[0.08454351872205734],
        "close":[0.11049100011587143],
        "volume":[11099804800]
      }],
      "adjclose":[{"adjclose":[0.0276227500289678575]}]
    },
    "events":{
      "dividends":{
        "1704067200":{"date":1704067200,"amount":0.1234567890123456789012345678}
      },
      "capitalGains":{
        "1704153600":{"date":1704153600,"amount":0.2345678901234567890123456789}
      }
    }
  }],"error":null}
}"#;

const ISSUE_TEN_BODY: &str = r#"{
  "chart":{"result":[{
    "meta":{"currency":"USD","symbol":"TEST","instrumentType":"EQUITY","priceHint":2},
    "timestamp":[470707200],
    "indicators":{
      "quote":[{
        "open":[0.13002200424671173],
        "high":[0.13895100355148315],
        "low":[0.08454351872205734],
        "close":[0.11049100011587143],
        "volume":[11099804800]
      }],
      "adjclose":[{"adjclose":[0.08454351872205734]}]
    }
  }],"error":null}
}"#;

fn decimal(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}

fn client(server: &MockServer) -> YfClient {
    YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap()
}

#[tokio::test]
async fn chart_preserves_exact_price_and_action_lexemes() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/TEST");
        then.status(200)
            .header("content-type", "application/json")
            .body(BODY);
    });

    let response = HistoryBuilder::new(&client(&server), "TEST")
        .auto_adjust(false)
        .fetch_full()
        .await
        .unwrap();

    mock.assert();
    let candle = &response.candles[0];
    assert_eq!(
        candle.ohlc.open.as_decimal(),
        &decimal("0.13002200424671173")
    );
    assert_eq!(
        candle.ohlc.high.as_decimal(),
        &decimal("0.13895100355148315")
    );
    assert_eq!(
        candle.ohlc.low.as_decimal(),
        &decimal("0.08454351872205734")
    );
    assert_eq!(
        candle.ohlc.close.as_decimal(),
        &decimal("0.11049100011587143")
    );
    assert_eq!(
        candle.close_unadj.as_ref().unwrap().as_decimal(),
        &decimal("0.11049100011587143")
    );
    assert!(response.actions.iter().any(|action| {
        matches!(
            action,
            Action::Dividend { amount, .. }
                if amount.amount() == decimal("0.1234567890123456789012345678")
        )
    }));
    assert!(response.actions.iter().any(|action| {
        matches!(
            action,
            Action::CapitalGain { gain, .. }
                if gain.amount() == decimal("0.2345678901234567890123456789")
        )
    }));
}

#[tokio::test]
async fn chart_auto_adjusts_with_decimal_arithmetic() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/TEST");
        then.status(200)
            .header("content-type", "application/json")
            .body(BODY);
    });

    let response = HistoryBuilder::new(&client(&server), "TEST")
        .auto_adjust(true)
        .fetch_full()
        .await
        .unwrap();

    mock.assert();
    let candle = &response.candles[0];
    assert_eq!(
        candle.ohlc.open.as_decimal(),
        &decimal("0.0325055010616779325")
    );
    assert_eq!(
        candle.ohlc.high.as_decimal(),
        &decimal("0.0347377508878707875")
    );
    assert_eq!(
        candle.ohlc.low.as_decimal(),
        &decimal("0.021135879680514335")
    );
    assert_eq!(
        candle.ohlc.close.as_decimal(),
        &decimal("0.0276227500289678575")
    );
    assert_eq!(
        candle.close_unadj.as_ref().unwrap().as_decimal(),
        &decimal("0.11049100011587143")
    );
}

#[tokio::test]
async fn chart_rounding_is_opt_in_covers_raw_close_and_leaves_actions_exact() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/TEST");
        then.status(200)
            .header("content-type", "application/json")
            .body(BODY);
    });

    let response = HistoryBuilder::new(&client(&server), "TEST")
        .auto_adjust(false)
        .rounding(true)
        .fetch_full()
        .await
        .unwrap();

    mock.assert();
    let candle = &response.candles[0];
    assert_eq!(candle.ohlc.open.as_decimal(), &decimal("0.13"));
    assert_eq!(candle.ohlc.high.as_decimal(), &decimal("0.14"));
    assert_eq!(candle.ohlc.low.as_decimal(), &decimal("0.08"));
    assert_eq!(candle.ohlc.close.as_decimal(), &decimal("0.11"));
    assert_eq!(
        candle.close_unadj.as_ref().unwrap().as_decimal(),
        &decimal("0.11")
    );
    assert!(response.actions.iter().any(|action| {
        matches!(
            action,
            Action::Dividend { amount, .. }
                if amount.amount() == decimal("0.1234567890123456789012345678")
        )
    }));
}

#[tokio::test]
async fn chart_issue_ten_adjustment_rounds_only_at_decimal_output() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/v8/finance/chart/TEST");
        then.status(200)
            .header("content-type", "application/json")
            .body(ISSUE_TEN_BODY);
    });

    let response = HistoryBuilder::new(&client(&server), "TEST")
        .auto_adjust(true)
        .fetch_full()
        .await
        .unwrap();

    mock.assert();
    let candle = &response.candles[0];
    assert_eq!(
        candle.ohlc.open.as_decimal(),
        &decimal("0.0994879016280374572098676566")
    );
    assert_eq!(
        candle.ohlc.high.as_decimal(),
        &decimal("0.1063200329247089571168970065")
    );
    assert_eq!(
        candle.ohlc.low.as_decimal(),
        &decimal("0.0646894910029884437241280919")
    );
    assert_eq!(
        candle.ohlc.close.as_decimal(),
        &decimal("0.08454351872205734")
    );
    assert_eq!(
        candle.close_unadj.as_ref().unwrap().as_decimal(),
        &decimal("0.11049100011587143")
    );
}
