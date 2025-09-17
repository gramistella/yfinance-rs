use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::core::conversions::*;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
async fn options_expirations_happy() {
    let server = MockServer::start();

    // Minimal expirations-only body
    let body = r#"{
      "optionChain": {
        "result": [{
          "underlyingSymbol":"AAPL",
          "expirationDates":[1737072000,1737676800],
          "hasMiniOptions": false,
          "options":[]
        }],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET).path("/v7/finance/options/AAPL");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder()
        .base_options_v7(Url::parse(&format!("{}/v7/finance/options/", server.base_url())).unwrap())
        .build()
        .unwrap();
    let t = Ticker::new(&client, "AAPL");

    let expiries = t.options().await.unwrap();
    mock.assert();

    assert_eq!(expiries, vec![1_737_072_000, 1_737_676_800]);
}

#[tokio::test]
async fn option_chain_for_specific_date() {
    let server = MockServer::start();

    // Body including one expiration "options" entry with one call and one put
    let body = r#"{
      "optionChain": {
        "result": [{
          "underlyingSymbol":"AAPL",
          "expirationDates":[1737072000,1737676800],
          "hasMiniOptions": false,
          "options": [{
            "expirationDate": 1737072000,
            "hasMiniOptions": false,
            "calls": [{
              "contractSymbol":"AAPL250117C00180000",
              "strike":180.0,
              "lastPrice":5.1,
              "bid":5.0,
              "ask":5.2,
              "volume":123,
              "openInterest":1000,
              "impliedVolatility":0.25,
              "inTheMoney":true
            }],
            "puts": [{
              "contractSymbol":"AAPL250117P00180000",
              "strike":180.0,
              "lastPrice":3.4,
              "bid":3.3,
              "ask":3.5,
              "volume":89,
              "openInterest":800,
              "impliedVolatility":0.27,
              "inTheMoney":false
            }]
          }]
        }],
        "error": null
      }
    }"#;

    let date = 1_737_072_000_i64;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/options/AAPL")
            .query_param("date", date.to_string());
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder()
        .base_options_v7(Url::parse(&format!("{}/v7/finance/options/", server.base_url())).unwrap())
        .build()
        .unwrap();
    let t = Ticker::new(&client, "AAPL");

    let chain = t.option_chain(Some(date)).await.unwrap();
    mock.assert();

    assert_eq!(chain.calls.len(), 1);
    assert_eq!(chain.puts.len(), 1);

    let c = &chain.calls[0];
    assert_eq!(c.contract_symbol, "AAPL250117C00180000");
    assert!((money_to_f64(&c.strike) - 180.0).abs() < 1e-9);
    assert_eq!(c.volume, Some(123));
    assert_eq!(c.open_interest, Some(1000));
    assert_eq!(c.implied_volatility, Some(0.25));
    assert!(c.in_the_money);
    assert_eq!(c.expiration.timestamp(), date);

    let p = &chain.puts[0];
    assert_eq!(p.contract_symbol, "AAPL250117P00180000");
    assert!((money_to_f64(p.price.as_ref().unwrap()) - 3.4).abs() < 1e-9);
    assert_eq!(p.expiration.timestamp(), date);
}

#[tokio::test]
async fn options_retry_with_crumb_on_403() {
    use httpmock::Method::GET;
    use httpmock::MockServer;
    use url::Url;
    use yfinance_rs::{Ticker, YfClient};

    let server = MockServer::start();

    // First call returns 403 (unauthorized) ONLY when the crumb is missing.
    let date = 1_737_072_000_i64;
    let first = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/options/MSFT")
            .query_param("date", date.to_string())
            .matches(|req| {
                // httpmock 0.7 exposes `query_params` as a nested Vec.
                // Reject the match if ANY "crumb" param is present.
                if let Some(group) = &req.query_params {
                    for (k, _) in group {
                        if k == "crumb" {
                            return false;
                        }
                    }
                }
                true
            });
        then.status(403);
    });

    // Cookie + crumb endpoints for ensure_credentials()
    let cookie = server.mock(|when, then| {
        when.method(GET).path("/consent");
        then.status(200).header(
            "set-cookie",
            "A=B; Max-Age=315360000; Domain=.yahoo.com; Path=/; Secure; SameSite=None",
        );
    });

    let crumb = server.mock(|when, then| {
        when.method(GET).path("/v1/test/getcrumb");
        then.status(200).body("crumb-value");
    });

    // Second attempt with ?crumb= should succeed
    let ok_body = r#"{
      "optionChain": {
        "result": [{
          "underlyingSymbol":"MSFT",
          "expirationDates":[1737072000],
          "options": [{
            "expirationDate": 1737072000,
            "calls": [],
            "puts": []
          }]
        }],
        "error": null
      }
    }"#;

    let second = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/options/MSFT")
            .query_param("date", date.to_string())
            .query_param("crumb", "crumb-value");
        then.status(200)
            .header("content-type", "application/json")
            .body(ok_body);
    });

    let client = YfClient::builder()
        .cookie_url(Url::parse(&format!("{}/consent", server.base_url())).unwrap())
        .crumb_url(Url::parse(&format!("{}/v1/test/getcrumb", server.base_url())).unwrap())
        .base_options_v7(Url::parse(&format!("{}/v7/finance/options/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let t = Ticker::new(&client, "MSFT");

    let chain = t.option_chain(Some(date)).await.unwrap();
    assert!(chain.calls.is_empty() && chain.puts.is_empty());

    first.assert();
    cookie.assert();
    crumb.assert();
    second.assert();
}
