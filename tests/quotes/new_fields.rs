use httpmock::Method::GET;
use httpmock::MockServer;
use paft::money::{Currency, IsoCurrency};
use rust_decimal::Decimal;
use url::Url;
use yfinance_rs::core::conversions::f64_to_money_with_currency;
use yfinance_rs::YfClient;

fn usd() -> Currency {
    Currency::Iso(IsoCurrency::USD)
}

/// Parse the AAPL v7 fixture and assert all new fields are correctly parsed.
#[tokio::test]
async fn quote_v7_new_fields_parsed_from_fixture() {
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", "AAPL");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("quote_v7", "AAPL", "json"));
    });

    let client = YfClient::builder()
        .base_quote_v7(
            Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap(),
        )
        .build()
        .unwrap();

    let quotes = yfinance_rs::QuotesBuilder::new(client)
        .symbols(["AAPL"])
        .fetch()
        .await
        .unwrap();

    assert_eq!(quotes.len(), 1);
    let q = &quotes[0];

    // open: regularMarketOpen = 269.275
    assert_eq!(q.open, Some(f64_to_money_with_currency(269.275, usd())));
    // day range: regularMarketDayHigh = 271.41, regularMarketDayLow = 267.11
    assert_eq!(q.day_range_high, Some(f64_to_money_with_currency(271.41, usd())));
    assert_eq!(q.day_range_low, Some(f64_to_money_with_currency(267.11, usd())));
    // 52-week range: fiftyTwoWeekHigh = 271.41, fiftyTwoWeekLow = 169.21
    assert_eq!(q.fifty_two_week_high, Some(f64_to_money_with_currency(271.41, usd())));
    assert_eq!(q.fifty_two_week_low, Some(f64_to_money_with_currency(169.21, usd())));
    // market cap: marketCap = 4002453389312
    assert_eq!(q.market_cap, Some(f64_to_money_with_currency(4_002_453_389_312.0, usd())));
    // shares outstanding: sharesOutstanding = 14840390000
    assert_eq!(q.shares_outstanding, Some(14_840_390_000u64));
    // eps_ttm: epsTrailingTwelveMonths = 6.59
    assert_eq!(q.eps_ttm, Some(f64_to_money_with_currency(6.59, usd())));
    // pe_ttm: trailingPE = 40.925644
    assert_eq!(q.pe_ttm, Decimal::try_from(40.925644_f64).ok());
    // dividend_yield: trailingAnnualDividendYield = 0.0037546468
    assert_eq!(q.dividend_yield, Decimal::try_from(0.0037546468_f64).ok());
    // average_volume: averageDailyVolume3Month = 54683671
    assert_eq!(q.average_volume, Some(54_683_671u64));
    // ex_dividend_date: dividendDate = 1755129600 → 2025-08-14
    assert_eq!(
        q.ex_dividend_date,
        chrono::DateTime::from_timestamp(1_755_129_600, 0).map(|dt| dt.date_naive())
    );
}
