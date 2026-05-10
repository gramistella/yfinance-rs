use httpmock::Method::GET;
use httpmock::MockServer;
use paft::money::{Currency, IsoCurrency};
use url::Url;
use yfinance_rs::core::conversions::f64_to_money_with_currency;
use yfinance_rs::{Ticker, YfClient};

fn make_client(server: &MockServer, sym: &str) -> (YfClient, Ticker) {
    let client = YfClient::builder()
        .base_timeseries(
            Url::parse(&format!(
                "{}/ws/fundamentals-timeseries/v1/finance/timeseries/",
                server.base_url()
            ))
            .unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, sym);
    (client, ticker)
}

/* ------------- income statement new fields ------------- */

#[tokio::test]
async fn income_statement_new_fields_populated() {
    let sym = "AAPL";
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture(
                "timeseries_income_statement_new_fields_annual",
                sym,
                "json",
            ));
    });

    let (_client, ticker) = make_client(&server, sym);
    let rows = ticker.income_stmt(None).await.unwrap();

    assert!(!rows.is_empty());
    let row = &rows[0];
    assert_eq!(
        row.interest_expense,
        Some(f64_to_money_with_currency(-3.93e9, Currency::Iso(IsoCurrency::USD)))
    );
    assert_eq!(
        row.tax_expense,
        Some(f64_to_money_with_currency(2.9749e10, Currency::Iso(IsoCurrency::USD)))
    );
    assert_eq!(
        row.depreciation_and_amortization,
        Some(f64_to_money_with_currency(1.1445e10, Currency::Iso(IsoCurrency::USD)))
    );
}

/* ------------- balance sheet new fields ------------- */

#[tokio::test]
async fn balance_sheet_new_fields_populated() {
    let sym = "MSFT";
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture(
                "timeseries_balance_sheet_new_fields_annual",
                sym,
                "json",
            ));
    });

    let (_client, ticker) = make_client(&server, sym);
    let rows = ticker.balance_sheet(None).await.unwrap();

    assert!(!rows.is_empty());
    let row = &rows[0];
    // Fixture values from timeseries_balance_sheet_new_fields_annual_MSFT.json
    assert_eq!(
        row.accounts_receivable,
        Some(f64_to_money_with_currency(4.5e10, Currency::Iso(IsoCurrency::USD)))
    );
    assert_eq!(
        row.inventory,
        Some(f64_to_money_with_currency(2.5e9, Currency::Iso(IsoCurrency::USD)))
    );
    assert_eq!(
        row.accounts_payable,
        Some(f64_to_money_with_currency(1.8e10, Currency::Iso(IsoCurrency::USD)))
    );
    assert_eq!(
        row.current_assets,
        Some(f64_to_money_with_currency(1.5e11, Currency::Iso(IsoCurrency::USD)))
    );
    assert_eq!(
        row.current_liabilities,
        Some(f64_to_money_with_currency(9.0e10, Currency::Iso(IsoCurrency::USD)))
    );
    assert_eq!(
        row.net_ppe,
        Some(f64_to_money_with_currency(1.2e11, Currency::Iso(IsoCurrency::USD)))
    );
    assert_eq!(
        row.intangible_assets,
        Some(f64_to_money_with_currency(7.0e10, Currency::Iso(IsoCurrency::USD)))
    );
}

/* ------------- cashflow new fields ------------- */

#[tokio::test]
async fn cashflow_new_fields_populated() {
    let sym = "GOOGL";
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture(
                "timeseries_cash_flow_new_fields_annual",
                sym,
                "json",
            ));
    });

    let (_client, ticker) = make_client(&server, sym);
    let rows = ticker.cashflow(None).await.unwrap();

    assert!(!rows.is_empty());
    let row = &rows[0];
    assert_eq!(
        row.depreciation_and_amortization,
        Some(f64_to_money_with_currency(1.4e10, Currency::Iso(IsoCurrency::USD)))
    );
}
