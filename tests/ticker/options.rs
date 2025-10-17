use serde_json::Value;
use url::Url;
use yfinance_rs::core::conversions::money_to_currency_str;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
async fn options_expirations_happy() {
    let server = crate::common::setup_server();
    let symbol = "AAPL";

    let mock = crate::common::mock_options_v7(&server, symbol);

    let client = YfClient::builder()
        .base_options_v7(Url::parse(&format!("{}/v7/finance/options/", server.base_url())).unwrap())
        .build()
        .unwrap();
    let t = Ticker::new(&client, symbol);

    let expiries = t.options().await.unwrap();
    mock.assert();

    assert!(
        !expiries.is_empty(),
        "record {symbol} options fixtures first via YF_RECORD=1 cargo test --test ticker -- options"
    );
}

#[tokio::test]
async fn option_chain_for_specific_date() {
    let server = crate::common::setup_server();
    let symbol = "AAPL";

    let exp_mock = crate::common::mock_options_v7(&server, symbol);
    let quote_mock = crate::common::mock_quote_v7(&server, symbol);

    let client = YfClient::builder()
        .base_options_v7(Url::parse(&format!("{}/v7/finance/options/", server.base_url())).unwrap())
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();
    let t = Ticker::new(&client, symbol);

    let expiries = t.options().await.unwrap();
    exp_mock.assert();

    assert!(
        !expiries.is_empty(),
        "record {symbol} options fixtures first via YF_RECORD=1 cargo test --test ticker -- options"
    );

    let date = expiries[0];
    let chain_mock = crate::common::mock_options_v7_for_date(&server, symbol, date);

    let chain = t.option_chain(Some(date)).await.unwrap();
    chain_mock.assert();
    assert_eq!(
        quote_mock.calls(),
        0,
        "options currency should prevent quote fallback"
    );

    assert!(
        !chain.calls.is_empty(),
        "recorded {symbol} chain should include call contracts"
    );
    assert!(
        !chain.puts.is_empty(),
        "recorded {symbol} chain should include put contracts"
    );

    let c = &chain.calls[0];
    assert_eq!(money_to_currency_str(&c.strike).as_deref(), Some("USD"));
    assert_eq!(c.expiration_at.unwrap().timestamp(), date);

    let p = &chain.puts[0];
    if let Some(price) = p.price.as_ref() {
        assert_eq!(money_to_currency_str(price).as_deref(), Some("USD"));
    }
    assert_eq!(p.expiration_at.unwrap().timestamp(), date);
}

#[tokio::test]
async fn option_chain_currency_fallback_fetches_quote() {
    let server = crate::common::setup_server();
    let symbol = "AAPL";

    assert_fixture_present(symbol);

    let mut base_json = load_options_json(symbol);
    let expiries = extract_expiration_dates(&base_json);
    assert!(
        !expiries.is_empty(),
        "recorded {symbol} options fixture missing expiration dates"
    );
    strip_quote_currency(&mut base_json);
    let base_payload = base_json.to_string();

    let date = expiries[0];
    let fixture_key = format!("{symbol}_{date}");
    assert_fixture_present(&fixture_key);

    let mut dated_json = load_options_json(&fixture_key);
    strip_quote_currency(&mut dated_json);
    let dated_payload = dated_json.to_string();

    let base_mock = mock_base_options_request(&server, symbol, base_payload);
    let chain_mock = mock_dated_options_request(&server, symbol, date, dated_payload);
    let quote_mock = crate::common::mock_quote_v7(&server, symbol);

    let client = YfClient::builder()
        .base_options_v7(Url::parse(&format!("{}/v7/finance/options/", server.base_url())).unwrap())
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, symbol);

    let expiries_resp = ticker.options().await.unwrap();
    base_mock.assert();
    assert_eq!(expiries_resp, expiries);

    let chain = ticker.option_chain(Some(date)).await.unwrap();

    chain_mock.assert();
    quote_mock.assert();

    assert!(
        quote_mock.calls() >= 1,
        "fallback should hit quote endpoint at least once"
    );

    let combined = chain
        .calls
        .iter()
        .chain(chain.puts.iter())
        .collect::<Vec<_>>();
    assert!(
        !combined.is_empty(),
        "recorded chain for {symbol} should include contracts"
    );

    for contract in combined {
        assert_eq!(
            money_to_currency_str(&contract.strike).as_deref(),
            Some("USD")
        );
        assert_eq!(contract.expiration_at.unwrap().timestamp(), date);
    }
}

fn assert_fixture_present(id: &str) {
    assert!(
        crate::common::fixture_exists("options_v7", id, "json"),
        "record {id} options fixtures via YF_RECORD=1 cargo test --test ticker -- options"
    );
}

fn load_options_json(id: &str) -> Value {
    let body = crate::common::fixture("options_v7", id, "json");
    serde_json::from_str(&body).expect("options fixture json")
}

fn extract_expiration_dates(json: &Value) -> Vec<i64> {
    json["optionChain"]["result"][0]["expirationDates"]
        .as_array()
        .expect("expirationDates array")
        .iter()
        .map(|v| v.as_i64().expect("epoch"))
        .collect()
}

fn strip_quote_currency(json: &mut Value) {
    if let Some(obj) = json
        .get_mut("optionChain")
        .and_then(|oc| oc.get_mut("result"))
        .and_then(|arr| arr.get_mut(0))
        .and_then(|node| node.get_mut("quote"))
        .and_then(|quote| quote.as_object_mut())
    {
        obj.remove("currency");
    }
}

fn mock_base_options_request<'a>(
    server: &'a httpmock::MockServer,
    symbol: &str,
    payload: String,
) -> httpmock::Mock<'a> {
    let symbol = symbol.to_string();
    let body = payload;
    server.mock(move |when, then| {
        when.method(httpmock::Method::GET)
            .path(format!("/v7/finance/options/{symbol}"))
            .is_true(|req| !req.query_params().iter().any(|(k, _)| k == "date"));
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    })
}

fn mock_dated_options_request<'a>(
    server: &'a httpmock::MockServer,
    symbol: &str,
    date: i64,
    payload: String,
) -> httpmock::Mock<'a> {
    let symbol = symbol.to_string();
    let body = payload;
    server.mock(move |when, then| {
        when.method(httpmock::Method::GET)
            .path(format!("/v7/finance/options/{symbol}"))
            .query_param("date", date.to_string());
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    })
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
            .is_true(|req| !req.query_params().iter().any(|(k, _)| k == "crumb"));
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
          "quote": {
            "currency": "USD"
          },
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
