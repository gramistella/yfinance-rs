/// Live recorder to create tests/fixtures/quote_v7_MULTI.json via internal::net::get_text.
/// Run it explicitly with recording turned on:
///   YF_RECORD=1 cargo test --test quotes -- --ignored record_multi_quotes_live
use url::Url;

#[tokio::test]
#[ignore]
async fn record_multi_quotes_live() {
    let mut client = yfinance_rs::YfClient::builder().build().unwrap();

    // Use the real base URL; this will record to quote_v7_MULTI.json
    let _ = yfinance_rs::QuotesBuilder::new(&mut client)
        .unwrap()
        .quote_base(Url::parse("https://query1.finance.yahoo.com/v7/finance/quote").unwrap())
        .symbols(["AAPL", "MSFT"])
        .fetch()
        .await
        .unwrap();
}
