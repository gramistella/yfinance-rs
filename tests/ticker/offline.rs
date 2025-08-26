use url::Url;
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
async fn offline_quote_uses_recorded_fixture() {
    let server = crate::common::setup_server();
    let sym = "AAPL";
    let mock = crate::common::mock_quote_v7(&server, sym);

    let client = YfClient::builder().base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap(),).build().unwrap();
    let t = Ticker::new(
        client,
        sym,
    )
    .unwrap();

    let q = t.quote().await.unwrap();
    mock.assert();

    assert_eq!(q.symbol, sym);
    // Don’t assert exact prices — fixtures will be from your latest recording
    assert!(q.currency.is_some());
}

#[tokio::test]
async fn offline_options_uses_recorded_fixtures() {
    let server = crate::common::setup_server();
    let sym = "AAPL";

    // Expirations (no date)
    let mock_exp = crate::common::mock_options_v7(&server, sym);

    let client = YfClient::builder()
                        .base_options_v7(Url::parse(&format!("{}/v7/finance/options/", server.base_url())).unwrap(),).build().unwrap();
    let t = Ticker::new(
        client,
        sym,
    )
    .unwrap();

    let expiries = t.options().await.unwrap();
    mock_exp.assert();

    assert!(
        !expiries.is_empty(),
        "record expirations via YF_RECORD=1 first"
    );

    // Chain for the first date (date-scoped fixture)
    let d = expiries[0];
    let mock_chain = crate::common::mock_options_v7_for_date(&server, sym, d);

    let chain = t.option_chain(Some(d)).await.unwrap();
    mock_chain.assert();

    // Contracts, if present, should carry the requested expiration
    for c in chain.calls.iter().chain(chain.puts.iter()) {
        assert_eq!(c.expiration, d);
    }
}
