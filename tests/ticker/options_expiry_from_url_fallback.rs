use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
async fn option_chain_expiration_falls_back_to_url_date() {
    let server = MockServer::start();
    let date = 1737072000_i64;

    // Note: "expirationDate" is deliberately omitted from the payload.
    let body = r#"{
      "optionChain": {
        "result": [{
          "options": [{
            "calls": [{
              "contractSymbol":"AAPL250117C00180000",
              "strike":180.0
            }],
            "puts": [{
              "contractSymbol":"AAPL250117P00180000",
              "strike":180.0
            }]
          }]
        }],
        "error": null
      }
    }"#;

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

    let t = Ticker::new(client, "AAPL");

    let chain = t.option_chain(Some(date)).await.unwrap();

    mock.assert();

    assert!(!chain.calls.is_empty() && !chain.puts.is_empty());
    assert!(
        chain
            .calls
            .iter()
            .chain(chain.puts.iter())
            .all(|c| c.expiration == date),
        "expiration must fall back to 'date' query param"
    );
}
