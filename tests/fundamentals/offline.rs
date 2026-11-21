use httpmock::Method::GET;
use httpmock::MockServer;
use paft::domain::ReportingPeriod;
use paft::fundamentals::statements::{BalanceSheetRow, CashflowRow, IncomeStatementRow};
use paft::money::{Currency, IsoCurrency, Money};
use url::Url;
use yfinance_rs::core::conversions::{i64_to_date, money_from_f64, money_to_f64};
use yfinance_rs::{FundamentalsBuilder, Ticker, YfClient};

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

    let response = FundamentalsBuilder::new(&client, sym)
        .income_statement_with_diagnostics(true, None)
        .await
        .unwrap();

    mock.assert();
    assert!(response.diagnostics.is_empty());
    let rows = response.data;
    assert!(!rows.is_empty(), "record with YF_RECORD=1 first");
}

async fn assert_offline_income_maps_net_income_from_continuing_operations(
    sym: &str,
    quarterly: bool,
) {
    let prefix = if quarterly { "quarterly" } else { "annual" };
    let body = fixture(&format!("timeseries_income_statement_{prefix}"), sym);
    let expected = fixture_series_newest_first(&body, prefix, "NetIncomeFromContinuingOperations");
    assert!(
        !expected.is_empty(),
        "fixture lacks {prefix}NetIncomeFromContinuingOperations; record with YF_RECORD=1 first"
    );

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_includes("type", format!("{prefix}NetIncomeFromContinuingOperations"))
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
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

    let response = FundamentalsBuilder::new(&client, sym)
        .income_statement_with_diagnostics(quarterly, None)
        .await
        .unwrap();

    mock.assert();
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    let rows: Vec<IncomeStatementRow> = response.data;
    let actual: Vec<(ReportingPeriod, Money)> = rows
        .iter()
        .filter_map(|r| {
            Some((
                r.period.clone(),
                r.net_income_from_continuing_operations.clone()?,
            ))
        })
        .collect();
    assert_eq!(actual, expected);

    // Other new paft income statement fields are not fetched yet.
    for row in &rows {
        assert_eq!(row.cost_of_revenue, None);
        assert_eq!(row.research_and_development, None);
        assert_eq!(row.selling_general_and_administrative, None);
        assert_eq!(row.operating_expenses, None);
        assert_eq!(row.interest_income, None);
        assert_eq!(row.ebit, None);
        assert_eq!(row.ebitda, None);
        assert_eq!(row.pretax_income, None);
        assert_eq!(row.net_income_common_stockholders, None);
        assert_eq!(row.basic_eps, None);
        assert_eq!(row.diluted_eps, None);
        assert_eq!(row.basic_average_shares, None);
        assert_eq!(row.diluted_average_shares, None);
    }
}

#[tokio::test]
async fn offline_income_annual_maps_net_income_from_continuing_operations() {
    assert_offline_income_maps_net_income_from_continuing_operations("AAPL", false).await;
}

#[tokio::test]
async fn offline_income_quarterly_maps_net_income_from_continuing_operations() {
    assert_offline_income_maps_net_income_from_continuing_operations("AAPL", true).await;
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

/// `(period, reported value)` points of `{prefix}{field}` in a recorded
/// timeseries fixture, newest first (the order yfinance-rs returns statement rows in).
fn fixture_series_newest_first(
    body: &str,
    prefix: &str,
    field: &str,
) -> Vec<(ReportingPeriod, Money)> {
    let key = format!("{prefix}{field}");
    let raw: serde_json::Value = serde_json::from_str(body).unwrap();
    let mut points: Vec<(i64, f64)> = raw["timeseries"]["result"]
        .as_array()
        .expect("timeseries fixture should contain results")
        .iter()
        .filter_map(|item| Some((item["timestamp"].as_array()?, item[&key].as_array()?)))
        .flat_map(|(timestamps, values)| {
            timestamps.iter().zip(values).filter_map(|(ts, value)| {
                Some((ts.as_i64()?, value["reportedValue"]["raw"].as_f64()?))
            })
        })
        .collect();
    points.sort_by_key(|(ts, _)| std::cmp::Reverse(*ts));
    points
        .into_iter()
        .map(|(ts, value)| {
            let period = ReportingPeriod::date(i64_to_date(ts).expect("fixture timestamp"))
                .expect("fixture period");
            let money =
                money_from_f64(value, Currency::Iso(IsoCurrency::USD)).expect("fixture value");
            (period, money)
        })
        .collect()
}

async fn assert_offline_balance_sheet_maps_current_debt(sym: &str, quarterly: bool) {
    let prefix = if quarterly { "quarterly" } else { "annual" };
    let body = fixture(&format!("timeseries_balance_sheet_{prefix}"), sym);
    let expected = fixture_series_newest_first(&body, prefix, "CurrentDebt");
    assert!(
        !expected.is_empty(),
        "fixture lacks {prefix}CurrentDebt; record with YF_RECORD=1 first"
    );

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_includes("type", format!("{prefix}CurrentDebt"))
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
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

    let response = FundamentalsBuilder::new(&client, sym)
        .balance_sheet_with_diagnostics(quarterly, None)
        .await
        .unwrap();

    mock.assert();
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    let rows: Vec<BalanceSheetRow> = response.data;
    let actual: Vec<(ReportingPeriod, Money)> = rows
        .iter()
        .filter_map(|r| Some((r.period.clone(), r.current_debt.clone()?)))
        .collect();
    assert_eq!(actual, expected);

    // Other new paft balance sheet fields are not fetched yet.
    for row in &rows {
        assert_eq!(row.total_debt, None);
        assert_eq!(row.cash_and_short_term_investments, None);
        assert_eq!(row.other_current_assets, None);
        assert_eq!(row.other_current_liabilities, None);
        assert_eq!(row.retained_earnings, None);
        assert_eq!(row.common_stock, None);
        assert_eq!(row.treasury_stock, None);
        assert_eq!(row.minority_interest, None);
        assert_eq!(row.working_capital, None);
        assert_eq!(row.tangible_book_value, None);
    }
}

#[tokio::test]
async fn offline_balance_sheet_annual_maps_current_debt() {
    assert_offline_balance_sheet_maps_current_debt("MSFT", false).await;
}

#[tokio::test]
async fn offline_balance_sheet_quarterly_maps_current_debt() {
    assert_offline_balance_sheet_maps_current_debt("MSFT", true).await;
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

async fn assert_offline_cashflow_maps_cash_dividends_paid(sym: &str, quarterly: bool) {
    let prefix = if quarterly { "quarterly" } else { "annual" };
    let body = fixture(&format!("timeseries_cash_flow_{prefix}"), sym);
    let expected = fixture_series_newest_first(&body, prefix, "CashDividendsPaid");
    assert!(
        !expected.is_empty(),
        "fixture lacks {prefix}CashDividendsPaid; record with YF_RECORD=1 first"
    );

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_includes("type", format!("{prefix}CashDividendsPaid"))
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(&body);
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

    let response = FundamentalsBuilder::new(&client, sym)
        .cashflow_with_diagnostics(quarterly, None)
        .await
        .unwrap();

    mock.assert();
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    let rows: Vec<CashflowRow> = response.data;
    let actual: Vec<(ReportingPeriod, Money)> = rows
        .iter()
        .filter_map(|r| Some((r.period.clone(), r.cash_dividends_paid.clone()?)))
        .collect();
    assert_eq!(actual, expected);

    // paft convention: dividends paid out are a cash outflow, so negative
    // (zero for years without dividends).
    for (period, paid) in &actual {
        assert!(
            money_to_f64(paid) <= 0.0,
            "{period}: expected non-positive cash_dividends_paid, got {paid}"
        );
    }
    assert!(
        actual.iter().any(|(_, paid)| money_to_f64(paid) < 0.0),
        "fixture should contain at least one dividend payment"
    );

    // Other new paft cash flow fields are not fetched yet.
    for row in &rows {
        assert_eq!(row.stock_based_compensation, None);
        assert_eq!(row.change_in_working_capital, None);
        assert_eq!(row.investing_cashflow, None);
        assert_eq!(row.financing_cashflow, None);
        assert_eq!(row.issuance_of_debt, None);
        assert_eq!(row.repayment_of_debt, None);
        assert_eq!(row.repurchase_of_capital_stock, None);
        assert_eq!(row.end_cash_position, None);
    }
}

#[tokio::test]
async fn offline_cashflow_annual_maps_cash_dividends_paid() {
    assert_offline_cashflow_maps_cash_dividends_paid("GOOGL", false).await;
}

#[tokio::test]
async fn offline_cashflow_quarterly_maps_cash_dividends_paid() {
    assert_offline_cashflow_maps_cash_dividends_paid("GOOGL", true).await;
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
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(&client, sym);
    let cal = t.calendar().await.unwrap();

    mock.assert();
    assert!(
        !cal.earnings_dates.is_empty()
            || cal.ex_dividend_date.is_some()
            || cal.dividend_payment_date.is_some(),
        "record with YF_RECORD=1 first"
    );
}
