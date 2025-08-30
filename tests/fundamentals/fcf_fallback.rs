use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{Ticker, YfClient};

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
        .preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(client, sym);
    let rows = t.cashflow().await.unwrap();

    mock.assert();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].operating_cashflow, Some(100.0));
    assert_eq!(rows[0].capital_expenditures, Some(-30.0));
    assert_eq!(
        rows[0].free_cash_flow,
        Some(70.0),
        "fcf = ocf + capex (where capex is negative)"
    );
    assert_eq!(rows[0].net_income, Some(65.0));
}
