use crate::common::{mock_quote_v7_multi, setup_server};
use std::path::Path;
use url::Url;

#[tokio::test]
async fn offline_multi_quotes_uses_recorded_fixture() {
    // Skip if the recorded fixture isn't present; you must run the live recorder first.
    let fixture = Path::new("tests/fixtures/quote_v7_MULTI.json");
    if !fixture.exists() {
        eprintln!(
            "skipping offline test: missing {}. run the live recorder with YF_RECORD=1 first.",
            fixture.display()
        );
        return;
    }

    let server = setup_server();
    let _mock = mock_quote_v7_multi(&server, "AAPL,MSFT");

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let base = Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap();

    let quotes = yfinance_rs::QuotesBuilder::new(client)
        .unwrap()
        .quote_base(base)
        .symbols(["AAPL", "MSFT"])
        .fetch()
        .await
        .unwrap();

    // Sanity against the recorded fixture
    let syms: Vec<_> = quotes.iter().map(|q| q.symbol.as_str()).collect();
    assert!(syms.contains(&"AAPL"));
    assert!(syms.contains(&"MSFT"));
}
