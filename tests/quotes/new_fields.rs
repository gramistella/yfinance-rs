use httpmock::Method::GET;
use httpmock::MockServer;
use paft::Decimal;
use url::Url;
use yfinance_rs::YfClient;

fn json_decimal(value: &serde_json::Value) -> Decimal {
    value
        .as_number()
        .expect("fixture decimal")
        .to_string()
        .parse()
        .expect("valid fixture decimal")
}

#[tokio::test]
async fn quote_v7_bid_ask_are_mapped_to_book_levels() {
    let server = MockServer::start();
    let fixture = crate::common::fixture("quote_v7", "AAPL", "json");
    let raw: serde_json::Value = serde_json::from_str(&fixture).unwrap();
    let raw_quote = raw["quoteResponse"]["result"]
        .as_array()
        .and_then(|quotes| quotes.first())
        .expect("quote fixture should contain AAPL");
    let expected_bid = json_decimal(&raw_quote["bid"]);
    let expected_bid_size = raw_quote["bidSize"].as_u64().expect("fixture bid size");
    let expected_ask = json_decimal(&raw_quote["ask"]);
    let expected_ask_size = raw_quote["askSize"].as_u64().expect("fixture ask size");

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture);
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .build()
        .unwrap();

    let quotes = yfinance_rs::QuotesBuilder::new(&client)
        .symbols(["AAPL"])
        .fetch()
        .await
        .unwrap();

    mock.assert();
    let quote = quotes.first().expect("quote fixture should contain AAPL");
    let bid = quote.bid.as_ref().expect("bid should be mapped");
    let ask = quote.ask.as_ref().expect("ask should be mapped");

    assert_eq!(bid.price.as_decimal(), &expected_bid);
    assert_eq!(
        bid.size.as_ref().map(ToString::to_string),
        Some(expected_bid_size.to_string())
    );
    assert_eq!(ask.price.as_decimal(), &expected_ask);
    assert_eq!(
        ask.size.as_ref().map(ToString::to_string),
        Some(expected_ask_size.to_string())
    );

    #[cfg(feature = "dataframe")]
    {
        use yfinance_rs::ToDataFrame;

        let df = quote.to_dataframe().unwrap();
        assert_eq!(df.height(), 1);
    }
}
