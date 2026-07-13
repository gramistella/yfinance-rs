use httpmock::Method::GET;
use httpmock::MockServer;
use paft::Decimal;
use paft::money::{Currency, IsoCurrency, Money};
use url::Url;
use yfinance_rs::{FundamentalsBuilder, Ticker, YfClient};

fn date_from_ts(timestamp: i64) -> chrono::NaiveDate {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .unwrap()
        .date_naive()
}

fn make_ticker(server: &MockServer, symbol: &str) -> Ticker {
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

    Ticker::new(&client, symbol)
}

fn usd(value: i64) -> Money {
    Money::new(Decimal::from(value), Currency::Iso(IsoCurrency::USD))
        .expect("known-good USD literal")
}

fn usd_i64(value: i64) -> Money {
    Money::new(paft::Decimal::from(value), Currency::Iso(IsoCurrency::USD))
        .expect("known-good USD literal")
}

#[tokio::test]
async fn calendar_maps_dividend_dates_to_distinct_paft_fields() {
    let server = MockServer::start();
    let symbol = "DIVS";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{symbol}"))
            .query_param("modules", "calendarEvents")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture(
                "fundamentals_api_calendarEvents",
                symbol,
                "json",
            ));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, symbol);
    let calendar = ticker.calendar().await.unwrap();

    mock.assert();
    assert_eq!(calendar.ex_dividend_date, Some(date_from_ts(1_758_499_200)));
    assert_eq!(
        calendar.dividend_payment_date,
        Some(date_from_ts(1_759_104_000))
    );
}

#[tokio::test]
async fn income_statement_accepts_timeseries_items_without_meta() {
    let server = MockServer::start();
    let symbol = "NOMETA";
    let body = r#"{
      "timeseries": {
        "result": [
          {
            "timestamp": [1727654400],
            "annualTotalRevenue": [{"reportedValue": {"raw": 1000}}]
          }
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let ticker = make_ticker(&server, symbol);
    let rows = ticker
        .income_stmt(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    let row = rows.first().expect("statement row should be present");
    assert_eq!(row.total_revenue, Some(usd_i64(1000)));
}

#[tokio::test]
async fn income_statement_skips_metadata_only_timeseries_items_losslessly() {
    let server = MockServer::start();
    let symbol = "METADATAONLY";
    let body = r#"{
      "timeseries": {
        "result": [
          { "meta": { "symbol": ["AAPL"], "type": ["quarterlyInterestExpense"] } },
          {
            "timestamp": [1727654400],
            "annualTotalRevenue": [{"reportedValue": {"raw": 1000}}]
          }
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
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

    let response = FundamentalsBuilder::new(&client, symbol)
        .income_statement_with_diagnostics(false, Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    assert!(response.diagnostics.is_empty());
    let row = response
        .data
        .first()
        .expect("statement row should be present");
    assert_eq!(row.total_revenue, Some(usd_i64(1000)));
}

#[tokio::test]
async fn income_statement_new_fields_are_mapped_to_paft_names() {
    let server = MockServer::start();
    let symbol = "AAPL";
    let body = r#"{
      "timeseries": {
        "result": [
          {"meta": {}, "timestamp": [1727654400], "annualInterestExpense": [{"reportedValue": {"raw": -3930000000}}]},
          {"meta": {}, "timestamp": [1727654400], "annualTaxProvision": [{"reportedValue": {"raw": 29749000000}}]},
          {"meta": {}, "timestamp": [1727654400], "annualDepreciationAndAmortization": [{"reportedValue": {"raw": 11445000000}}]}
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let ticker = make_ticker(&server, symbol);
    let rows = ticker
        .income_stmt(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    let row = rows.first().expect("statement row should be present");
    assert_eq!(row.interest_expense, Some(usd(-3_930_000_000)));
    assert_eq!(row.income_tax_expense, Some(usd(29_749_000_000)));
    assert_eq!(row.depreciation_and_amortization, Some(usd(11_445_000_000)));
}

#[tokio::test]
async fn income_statement_preserves_large_integer_statement_values() {
    let server = MockServer::start();
    let symbol = "BIG";
    let exact = 9_007_199_254_740_993_i64;
    let body = format!(
        r#"{{
      "timeseries": {{
        "result": [
          {{"meta": {{}}, "timestamp": [1727654400], "annualTotalRevenue": [{{"reportedValue": {{"raw": {exact}}}}}]}}
        ],
        "error": null
      }}
    }}"#
    );

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let ticker = make_ticker(&server, symbol);
    let rows = ticker
        .income_stmt(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    let row = rows.first().expect("statement row should be present");
    assert_eq!(row.total_revenue, Some(usd_i64(exact)));
}

#[tokio::test]
async fn income_statement_processes_all_fields_in_grouped_timeseries_item() {
    let server = MockServer::start();
    let symbol = "GROUPED";
    let body = r#"{
      "timeseries": {
        "result": [
          {
            "meta": {},
            "timestamp": [1727654400],
            "annualTotalRevenue": [{"reportedValue": {"raw": 1000}}],
            "annualNetIncome": [{"reportedValue": {"raw": 250}}]
          }
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let ticker = make_ticker(&server, symbol);
    let rows = ticker
        .income_stmt(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    let row = rows.first().expect("statement row should be present");
    assert_eq!(row.total_revenue, Some(usd_i64(1000)));
    assert_eq!(row.net_income, Some(usd_i64(250)));
}

#[tokio::test]
async fn statement_values_without_reported_raw_do_not_create_rows() {
    let server = MockServer::start();
    let symbol = "EMPTYRAW";

    let income_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param("symbol", symbol)
            .query_param(
                "type",
                "annualTotalRevenue,annualGrossProfit,annualOperatingIncome,annualNetIncome,annualInterestExpense,annualTaxProvision,annualDepreciationAndAmortization",
            )
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200).header("content-type", "application/json").body(
            r#"{
              "timeseries": {
                "result": [{
                  "meta": {},
                  "timestamp": [1727654400],
                  "annualTotalRevenue": [{ "reportedValue": {} }],
                  "annualNetIncome": [{}]
                }],
                "error": null
              }
            }"#,
        );
    });

    let balance_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param("symbol", symbol)
            .query_param(
                "type",
                "annualTotalAssets,annualTotalLiabilitiesNetMinorityInterest,annualStockholdersEquity,annualCashAndCashEquivalents,annualLongTermDebt,annualOrdinarySharesNumber,annualCurrentAssets,annualCurrentLiabilities,annualAccountsReceivable,annualInventory,annualAccountsPayable,annualNetPPE,annualGoodwill,annualOtherIntangibleAssets",
            )
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200).header("content-type", "application/json").body(
            r#"{
              "timeseries": {
                "result": [{
                  "meta": {},
                  "timestamp": [1727654400],
                  "annualTotalAssets": [{ "reportedValue": {} }],
                  "annualOrdinarySharesNumber": [{ "reportedValue": {} }]
                }],
                "error": null
              }
            }"#,
        );
    });

    let cashflow_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param("symbol", symbol)
            .query_param(
                "type",
                "annualOperatingCashFlow,annualCapitalExpenditure,annualFreeCashFlow,annualNetIncome,annualDepreciationAndAmortization",
            )
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200).header("content-type", "application/json").body(
            r#"{
              "timeseries": {
                "result": [{
                  "meta": {},
                  "timestamp": [1727654400],
                  "annualOperatingCashFlow": [{ "reportedValue": {} }],
                  "annualCapitalExpenditure": [{}]
                }],
                "error": null
              }
            }"#,
        );
    });

    let ticker = make_ticker(&server, symbol);

    assert!(ticker.income_stmt(None).await.unwrap().is_empty());
    assert!(ticker.balance_sheet(None).await.unwrap().is_empty());
    assert!(ticker.cashflow(None).await.unwrap().is_empty());

    income_mock.assert();
    balance_mock.assert();
    cashflow_mock.assert();
}

#[tokio::test]
async fn balance_sheet_new_fields_are_mapped_to_paft_names() {
    let server = MockServer::start();
    let symbol = "MSFT";
    let body = r#"{
      "timeseries": {
        "result": [
          {"meta": {}, "timestamp": [1751241600], "annualCurrentAssets": [{"reportedValue": {"raw": 150000000000}}]},
          {"meta": {}, "timestamp": [1751241600], "annualCurrentLiabilities": [{"reportedValue": {"raw": 90000000000}}]},
          {"meta": {}, "timestamp": [1751241600], "annualAccountsReceivable": [{"reportedValue": {"raw": 45000000000}}]},
          {"meta": {}, "timestamp": [1751241600], "annualInventory": [{"reportedValue": {"raw": 2500000000}}]},
          {"meta": {}, "timestamp": [1751241600], "annualAccountsPayable": [{"reportedValue": {"raw": 18000000000}}]},
          {"meta": {}, "timestamp": [1751241600], "annualNetPPE": [{"reportedValue": {"raw": 120000000000}}]},
          {"meta": {}, "timestamp": [1751241600], "annualGoodwill": [{"reportedValue": {"raw": 60000000000}}]},
          {"meta": {}, "timestamp": [1751241600], "annualOtherIntangibleAssets": [{"reportedValue": {"raw": 10000000000}}]}
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let ticker = make_ticker(&server, symbol);
    let rows = ticker
        .balance_sheet(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    let row = rows.first().expect("statement row should be present");
    assert_eq!(row.current_assets, Some(usd(150_000_000_000)));
    assert_eq!(row.current_liabilities, Some(usd(90_000_000_000)));
    assert_eq!(row.accounts_receivable, Some(usd(45_000_000_000)));
    assert_eq!(row.inventory, Some(usd(2_500_000_000)));
    assert_eq!(row.accounts_payable, Some(usd(18_000_000_000)));
    assert_eq!(row.net_property_plant_equipment, Some(usd(120_000_000_000)));
    assert_eq!(row.goodwill, Some(usd(60_000_000_000)));
    assert_eq!(row.intangible_assets, Some(usd(10_000_000_000)));
}

#[tokio::test]
async fn cashflow_new_fields_are_mapped_to_paft_names() {
    let server = MockServer::start();
    let symbol = "GOOGL";
    let body = r#"{
      "timeseries": {
        "result": [
          {"meta": {}, "timestamp": [1719705600], "annualDepreciationAndAmortization": [{"reportedValue": {"raw": 14000000000}}]}
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let ticker = make_ticker(&server, symbol);
    let rows = ticker
        .cashflow(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    let row = rows.first().expect("statement row should be present");
    assert_eq!(row.depreciation_and_amortization, Some(usd(14_000_000_000)));
}
