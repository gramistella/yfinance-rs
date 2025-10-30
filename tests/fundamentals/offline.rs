use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient};

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

/* ------------- income statement (quarterly) ------------- */

#[tokio::test]
async fn offline_income_quarterly_uses_recorded_fixture() {
    // Use AAPL for incomeStatementHistoryQuarterly (record first)
    let sym = "AAPL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("timeseries_income_statement_quarterly", sym));
    });

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

    let t = Ticker::new(&client, sym);
    let rows = t.quarterly_income_stmt(None).await.unwrap();

    mock.assert();
    assert!(!rows.is_empty(), "record with YF_RECORD=1 first");
}

/* ---------------- balance sheet (annual) ---------------- */

#[tokio::test]
async fn offline_balance_sheet_annual_uses_recorded_fixture() {
    let sym = "MSFT";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("timeseries_balance_sheet_annual", sym));
    });

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

    let t = Ticker::new(&client, sym);
    let rows = t.balance_sheet(None).await.unwrap();

    mock.assert();
    assert!(!rows.is_empty(), "record with YF_RECORD=1 first");
}

/* ---------------- cashflow (annual) ---------------- */

#[tokio::test]
async fn offline_cashflow_annual_uses_recorded_fixture() {
    let sym = "GOOGL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("timeseries_cash_flow_annual", sym));
    });

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

    let t = Ticker::new(&client, sym);
    let rows = t.cashflow(None).await.unwrap();

    mock.assert();
    assert!(!rows.is_empty(), "record with YF_RECORD=1 first");
}

/* ---------------- earnings ---------------- */

#[tokio::test]
async fn offline_earnings_uses_recorded_fixture() {
    // Use AMZN for earnings (record first)
    let sym = "AMZN";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("fundamentals_api_earnings", sym));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._api_preference(ApiPreference::ApiOnly)
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(&client, sym);
    let e = t.earnings(None).await.unwrap();

    mock.assert();
    assert!(
        !e.yearly.is_empty() || !e.quarterly.is_empty() || !e.quarterly_eps.is_empty(),
        "record with YF_RECORD=1 first"
    );
}

/* ---------------- calendar ---------------- */

#[tokio::test]
async fn offline_calendar_uses_recorded_fixture() {
    // Use META for calendarEvents (record first)
    let sym = "META";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "calendarEvents")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("fundamentals_api_calendarEvents", sym));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._api_preference(ApiPreference::ApiOnly)
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(&client, sym);
    let cal = t.calendar().await.unwrap();

    mock.assert();
    assert!(
        !cal.earnings_dates.is_empty()
            || cal.ex_dividend_date.is_some()
            || cal.ex_dividend_date.is_some(),
        "record with YF_RECORD=1 first"
    );
}
