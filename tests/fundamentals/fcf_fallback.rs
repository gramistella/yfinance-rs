use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient};

#[tokio::test]
async fn cashflow_computes_fcf_when_missing() {
    let server = MockServer::start();
    let sym = "GOOGL";

    let body = r#"{
      "quoteSummary": {
        "result": [{
          "cashflowStatementHistory": {
            "cashflowStatements": [{
              "endDate": { "raw": 1234567890 },
              "totalCashFromOperatingActivities": { "raw": 100.0 },
              "capitalExpenditures": { "raw": 30.0 },
              "freeCashflow": null,
              "netIncome": { "raw": 65.0 }
            }]
          }
        }],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "cashflowStatementHistory")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(client, sym);
    let rows = t.cashflow().await.unwrap();

    mock.assert();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].operating_cashflow, Some(100.0));
    assert_eq!(rows[0].capital_expenditures, Some(30.0));
    assert_eq!(rows[0].free_cash_flow, Some(70.0), "fcf = ocf - capex");
}
