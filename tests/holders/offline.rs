use httpmock::{Method::GET, Mock, MockServer};
use url::Url;
use yfinance_rs::{Ticker, YfClient};
use yfinance_rs::core::conversions::*;

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

fn setup_holders_mock<'a>(server: &'a MockServer, symbol: &'a str) -> Mock<'a> {
    let modules = "institutionOwnership,fundOwnership,majorHoldersBreakdown,insiderTransactions,insiderHolders,netSharePurchaseActivity";
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{symbol}"))
            .query_param("modules", modules)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("holders_api_institutionOwnership-fundOwnership-majorHoldersBreakdown-insiderTransactions-insiderHolders-netSharePurchaseActivity", symbol));
            })
}

#[tokio::test]
async fn offline_all_holders_from_fixture() {
    let sym = "AAPL";
    let server = MockServer::start();
    let mock = setup_holders_mock(&server, sym);

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    // Test each method; each will make an independent API call which the mock will serve.
    let t = Ticker::new(&client, sym);

    // Major Holders
    let major = t.major_holders().await.unwrap();
    assert!(!major.is_empty(), "major holders missing from fixture");
    assert!(
        major
            .iter()
            .any(|h| h.category.contains("Held by All Insider"))
    );
    assert!(
        major
            .iter()
            .any(|h| h.category.contains("Held by Institutions"))
    );

    // Institutional Holders
    let institutional = t.institutional_holders().await.unwrap();
    assert!(
        !institutional.is_empty(),
        "institutional holders missing from fixture"
    );
    assert!(institutional[0].shares > 0);

    // Mutual Fund Holders
    let mutual_fund = t.mutual_fund_holders().await.unwrap();
    assert!(
        !mutual_fund.is_empty(),
        "mutual fund holders missing from fixture"
    );
    assert!(money_to_f64(&mutual_fund[0].value) > 0.0);

    // Insider Roster
    let insider_roster = t.insider_roster_holders().await.unwrap();
    assert!(
        !insider_roster.is_empty(),
        "insider roster missing from fixture"
    );
    assert!(
        insider_roster
            .iter()
            .any(|h| h.name.to_lowercase().contains("cook"))
    );

    // Net Share Purchase Activity
    let net_purchase = t.net_share_purchase_activity().await.unwrap().unwrap();
    assert!(!net_purchase.period.is_empty());
    assert!(net_purchase.total_insider_shares > 0);

    // Insider Transactions (can be empty)
    let _insider_trans = t.insider_transactions().await.unwrap();

    // Verify the mock was hit for each of the 6 calls.
    mock.assert_hits(6);
}
