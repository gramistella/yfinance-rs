use httpmock::Method::GET;
use httpmock::MockServer;
use paft::Decimal;
use paft::money::{Currency, IsoCurrency, Money};
use url::Url;
use yfinance_rs::{FundamentalsBuilder, ProjectionIssue, Ticker, YfClient, YfError, YfWarning};

fn usd(value: i64) -> Money {
    Money::new(Decimal::from(value), Currency::Iso(IsoCurrency::USD))
        .expect("known-good USD literal")
}

fn gbp(value: i64) -> Money {
    Money::new(Decimal::from(value), Currency::Iso(IsoCurrency::GBP))
        .expect("known-good GBP literal")
}

#[tokio::test]
async fn cashflow_computes_fcf_when_missing() {
    let server = MockServer::start();
    let sym = "GOOGL";

    let body = r#"{
      "timeseries": {
        "result": [
          {
            "meta": { "type": ["annualOperatingCashFlow"] },
            "timestamp": [1234567890],
            "annualOperatingCashFlow": [{ "asOfDate": "2009-02-13", "periodType": "12M", "currencyCode": "USD", "reportedValue": {"raw": 100.0} }]
          },
          {
            "meta": { "type": ["annualCapitalExpenditure"] },
            "timestamp": [1234567890],
            "annualCapitalExpenditure": [{ "asOfDate": "2009-02-13", "periodType": "12M", "currencyCode": "USD", "reportedValue": {"raw": -30.0} }]
          },
          {
            "meta": { "type": ["annualNetIncome"] },
            "timestamp": [1234567890],
            "annualNetIncome": [{ "asOfDate": "2009-02-13", "periodType": "12M", "currencyCode": "USD", "reportedValue": {"raw": 65.0} }]
          }
        ],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
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

    let response = FundamentalsBuilder::new(&client, sym)
        .cashflow_with_diagnostics(false, None)
        .await
        .unwrap();
    let rows = response.data;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].operating_cashflow, Some(usd(100)));
    assert_eq!(rows[0].capital_expenditures, Some(usd(-30)));
    assert_eq!(
        rows[0].free_cash_flow,
        Some(usd(70)),
        "fcf = ocf + capex (where capex is negative)"
    );
    assert_eq!(rows[0].net_income, Some(usd(65)));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::RepairedData {
            item: "cash_flow",
            repair: "inferred missing free cash flow from operating cash flow and capital expenditure",
            ..
        }
    )));

    let err = FundamentalsBuilder::new(&client, sym)
        .strict()
        .cashflow(false, None)
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn statements_use_first_valid_currency_code_and_compare_units() {
    let server = MockServer::start();
    let sym = "MIXEDUNITS";
    let body = r#"{
      "timeseries": {
        "result": [{
          "meta": { "type": ["annualTotalRevenue"] },
          "timestamp": [1704067200, 1735689600, 1767225600],
          "annualTotalRevenue": [
            {
              "currencyCode": "!!!",
              "reportedValue": {"raw": 41.0}
            },
            {
              "currencyCode": "GBp",
              "reportedValue": {"raw": 42.0}
            },
            {
              "currencyCode": "GBX",
              "reportedValue": {"raw": 43.0}
            }
          ]
        }],
        "error": null
      }
    }"#;

    let timeseries_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{"quoteResponse":{"result":[{"symbol":"MIXEDUNITS","financialCurrency":"USD"}],"error":null}}"#,
            );
    });

    let client = YfClient::builder()
        .base_timeseries(
            Url::parse(&format!(
                "{}/ws/fundamentals-timeseries/v1/finance/timeseries/",
                server.base_url()
            ))
            .unwrap(),
        )
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .income_statement_with_diagnostics(false, None)
        .await
        .expect("best-effort invalid direct value should not poison later valid values");

    timeseries_mock.assert();
    assert_eq!(
        quote_mock.calls(),
        0,
        "valid same-payload currencyCode should avoid quote enrichment"
    );
    assert_eq!(response.data.len(), 3);

    let invalid = response
        .data
        .iter()
        .find(|row| row.period.to_string() == "2024-01-01")
        .unwrap();
    let first_valid = response
        .data
        .iter()
        .find(|row| row.period.to_string() == "2025-01-01")
        .unwrap();
    let equivalent = response
        .data
        .iter()
        .find(|row| row.period.to_string() == "2026-01-01")
        .unwrap();

    assert!(invalid.total_revenue.is_none());
    assert_eq!(first_valid.total_revenue, Some(gbp(42)));
    assert_eq!(equivalent.total_revenue, Some(gbp(43)));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "timeseries.reportedValue",
            key: Some(key),
            reason: ProjectionIssue::InvalidCurrency { code },
            ..
        } if key == "TotalRevenue@1704067200" && code == "!!!"
    )));
}

#[tokio::test]
async fn statements_use_timeseries_currency_code_before_enrichment() {
    let server = MockServer::start();
    let sym = "DIRECT";
    let body = r#"{
      "timeseries": {
        "result": [{
          "meta": { "type": ["annualTotalRevenue"] },
          "timestamp": [1704067200],
          "annualTotalRevenue": [{
            "asOfDate": "2024-01-01",
            "periodType": "12M",
            "currencyCode": "EUR",
            "reportedValue": {"raw": 42.0}
          }]
        }],
        "error": null
      }
    }"#;

    let timeseries_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{"quoteResponse":{"result":[{"symbol":"DIRECT","financialCurrency":"USD"}],"error":null}}"#,
            );
    });

    let client = YfClient::builder()
        .base_timeseries(
            Url::parse(&format!(
                "{}/ws/fundamentals-timeseries/v1/finance/timeseries/",
                server.base_url()
            ))
            .unwrap(),
        )
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let rows = Ticker::new(&client, sym).income_stmt(None).await.unwrap();

    timeseries_mock.assert();
    assert_eq!(quote_mock.calls(), 0);
    assert_eq!(
        rows.first()
            .and_then(|row| row.total_revenue.as_ref())
            .map(|money| money.currency().clone()),
        Some(Currency::Iso(IsoCurrency::EUR))
    );
}

#[tokio::test]
async fn conflicting_timeseries_currency_codes_omit_only_conflicting_values() {
    let server = MockServer::start();
    let sym = "MIXEDCURRENCY";
    let body = r#"{
      "timeseries": {
        "result": [{
          "meta": { "type": ["annualTotalRevenue"] },
          "timestamp": [1704067200, 1735689600],
          "annualTotalRevenue": [
            {
              "asOfDate": "2024-01-01",
              "periodType": "12M",
              "currencyCode": "USD",
              "reportedValue": {"raw": 42.0}
            },
            {
              "asOfDate": "2025-01-01",
              "periodType": "12M",
              "currencyCode": "EUR",
              "reportedValue": {"raw": 43.0}
            }
          ]
        }],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
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

    let response = FundamentalsBuilder::new(&client, sym)
        .income_statement_with_diagnostics(false, None)
        .await
        .expect("best-effort currency conflict should not abort the statement");

    assert_eq!(response.data.len(), 2);
    let valid = response
        .data
        .iter()
        .find(|row| row.period.to_string() == "2024-01-01")
        .unwrap();
    let conflicting = response
        .data
        .iter()
        .find(|row| row.period.to_string() == "2025-01-01")
        .unwrap();
    assert_eq!(
        valid.total_revenue.as_ref().map(Money::currency),
        Some(&Currency::Iso(IsoCurrency::USD))
    );
    assert!(conflicting.total_revenue.is_none());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "timeseries.reportedValue",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "currencyCode",
                details,
            },
            ..
        } if key == "TotalRevenue@1735689600"
            && details.contains("conflicting timeseries currencyCode values")
    )));

    let err = FundamentalsBuilder::new(&client, sym)
        .strict()
        .income_statement(false, None)
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn invalid_timeseries_currency_code_is_best_effort_diagnostic() {
    let server = MockServer::start();
    let sym = "BADCURRENCY";
    let body = r#"{
      "timeseries": {
        "result": [{
          "meta": { "type": ["annualTotalRevenue"] },
          "timestamp": [1704067200],
          "annualTotalRevenue": [{
            "asOfDate": "2024-01-01",
            "periodType": "12M",
            "currencyCode": "!!!",
            "reportedValue": {"raw": 42.0}
          }]
        }],
        "error": null
      }
    }"#;

    let timeseries_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{"quoteResponse":{"result":[{"symbol":"BADCURRENCY","financialCurrency":"USD"}],"error":null}}"#,
            );
    });

    let client = YfClient::builder()
        .base_timeseries(
            Url::parse(&format!(
                "{}/ws/fundamentals-timeseries/v1/finance/timeseries/",
                server.base_url()
            ))
            .unwrap(),
        )
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .income_statement_with_diagnostics(false, None)
        .await
        .expect("best-effort invalid direct timeseries currency should not fail");

    timeseries_mock.assert();
    assert_eq!(quote_mock.calls(), 0);
    assert_eq!(response.data.len(), 1);
    assert!(response.data[0].total_revenue.is_none());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "timeseries.reportedValue",
            reason: ProjectionIssue::InvalidCurrency { code },
            ..
        } if code == "!!!"
    )));
}

#[tokio::test]
async fn statements_omit_values_when_enriched_currency_is_invalid() {
    let server = MockServer::start();
    let sym = "BADENRICH";
    let body = r#"{
      "timeseries": {
        "result": [{
          "meta": { "type": ["annualTotalRevenue"] },
          "timestamp": [1704067200],
          "annualTotalRevenue": [{
            "asOfDate": "2024-01-01",
            "periodType": "12M",
            "reportedValue": {"raw": 42.0}
          }]
        }],
        "error": null
      }
    }"#;

    let timeseries_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{"quoteResponse":{"result":[{"symbol":"BADENRICH","quoteType":"EQUITY","financialCurrency":"!!!"}],"error":null}}"#,
            );
    });

    let client = YfClient::builder()
        .base_timeseries(
            Url::parse(&format!(
                "{}/ws/fundamentals-timeseries/v1/finance/timeseries/",
                server.base_url()
            ))
            .unwrap(),
        )
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .income_statement_with_diagnostics(false, None)
        .await
        .unwrap();

    timeseries_mock.assert();
    quote_mock.assert();
    assert_eq!(response.data.len(), 1);
    assert!(response.data[0].total_revenue.is_none());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "timeseries.reportedValue",
            reason: ProjectionIssue::InvalidCurrency { code },
            ..
        } if code == "!!!"
    )));
}
