use std::time::Duration;

use httpmock::{Method::GET, MockServer};
use paft::{
    Decimal,
    money::{Currency, IsoCurrency},
};
use url::Url;
use yfinance_rs::{FundamentalsBuilder, ProjectionIssue, Ticker, YfClient, YfError, YfWarning};

#[tokio::test]
async fn synthetic_earnings_eps_preserves_json_decimal_precision() {
    let server = MockServer::start();
    let sym = "PRECISE";
    let actual = "1.234567890123456789012345678";
    let estimate = "1.234567890123456789012345679";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{
                  "quoteSummary": {{
                    "result": [{{
                      "earnings": {{
                        "financialsChart": {{ "yearly": [], "quarterly": [] }},
                        "earningsChart": {{
                          "quarterly": [{{
                            "date": "2Q2025",
                            "actual": {{ "raw": {actual} }},
                            "estimate": {{ "raw": {estimate} }}
                          }}]
                        }}
                      }}
                    }}],
                    "error": null
                  }}
                }}"#
            ));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let earnings = Ticker::new(&client, sym)
        .earnings(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    assert_eq!(earnings.quarterly_eps.len(), 1);
    assert_eq!(
        earnings.quarterly_eps[0].actual.as_ref().unwrap().amount(),
        Decimal::from_str_exact(actual).unwrap()
    );
    assert_eq!(
        earnings.quarterly_eps[0]
            .estimate
            .as_ref()
            .unwrap()
            .amount(),
        Decimal::from_str_exact(estimate).unwrap()
    );
}

#[tokio::test]
async fn missing_earnings_module_is_provider_unavailable() {
    let server = MockServer::start();
    let sym = "NOEARN";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":[{}],"error":null}}"#);
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .earnings_with_diagnostics(None)
        .await
        .unwrap();

    assert!(response.data.yearly.is_empty());
    assert!(response.data.quarterly.is_empty());
    assert!(response.data.quarterly_eps.is_empty());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::ProviderFeatureUnavailable {
            feature: "earnings",
            reason: ProjectionIssue::ProviderUnavailable {
                feature: "earnings"
            },
            ..
        })
    ));

    let err = FundamentalsBuilder::new(&client, sym)
        .strict()
        .earnings(None)
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn quote_summary_http_error_maps_status_and_is_not_cached() {
    let server = MockServer::start();
    let sym = "AAPL";

    let mut error = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "crumb");
        then.status(500)
            .header("content-type", "text/html")
            .body("<html>server error</html>");
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .cache_ttl(Duration::from_secs(61))
        .retry_enabled(false)
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, sym);
    let err = ticker
        .earnings(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap_err();

    error.assert();
    match err {
        YfError::ServerError { status, url } => {
            assert_eq!(status, 500);
            assert!(url.contains("/v10/finance/quoteSummary/"));
            assert!(url.contains("crumb=REDACTED"));
            assert!(!url.contains("crumb=crumb"));
        }
        other => panic!("expected ServerError, got {other:?}"),
    }

    error.delete();
    let ok = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "earnings": {
                        "financialsChart": { "yearly": [], "quarterly": [] },
                        "earningsChart": { "quarterly": [] }
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });

    let earnings = ticker
        .earnings(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    ok.assert();
    assert!(earnings.yearly.is_empty());
    assert!(earnings.quarterly.is_empty());
    assert!(earnings.quarterly_eps.is_empty());
}

#[tokio::test]
async fn quote_summary_api_error_is_not_cached() {
    let server = MockServer::start();
    let sym = "AAPL";

    let mut error = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": null,
                    "error": { "description": "temporary yahoo error" }
                  }
                }"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .cache_ttl(Duration::from_secs(61))
        .retry_enabled(false)
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, sym);
    let err = ticker
        .earnings(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap_err();

    error.assert();
    assert!(
        matches!(err, YfError::Api(ref message) if message.contains("temporary yahoo error")),
        "expected Yahoo API error, got {err:?}"
    );

    error.delete();
    let ok = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earnings")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "earnings": {
                        "financialsChart": { "yearly": [], "quarterly": [] },
                        "earningsChart": { "quarterly": [] }
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });

    let earnings = ticker
        .earnings(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    ok.assert();
    assert!(earnings.yearly.is_empty());
    assert!(earnings.quarterly.is_empty());
    assert!(earnings.quarterly_eps.is_empty());
}

#[tokio::test]
async fn fundamentals_timeseries_http_error_maps_status_and_is_not_cached() {
    let server = MockServer::start();
    let sym = "MSFT";

    let mut rate_limited = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param("type", "annualOrdinarySharesNumber")
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(429)
            .header("content-type", "text/html")
            .body("<html>rate limited</html>");
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
        .cache_ttl(Duration::from_secs(61))
        .retry_enabled(false)
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, sym);
    let err = ticker.shares().await.unwrap_err();

    rate_limited.assert();
    match err {
        YfError::RateLimited { url } => {
            assert!(url.contains("/ws/fundamentals-timeseries/"));
            assert!(url.contains("crumb=REDACTED"));
            assert!(!url.contains("crumb=crumb"));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }

    rate_limited.delete();
    let ok = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param("type", "annualOrdinarySharesNumber")
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"timeseries":{"result":[]}}"#);
    });

    let shares = ticker.shares().await.unwrap();

    ok.assert();
    assert!(shares.is_empty());
}

#[tokio::test]
async fn fundamentals_timeseries_api_error_is_not_cached() {
    let server = MockServer::start();
    let sym = "MSFT";

    let mut api_error = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param("type", "annualOrdinarySharesNumber")
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "finance": {
                    "result": null,
                    "error": {
                      "code": "Bad Request",
                      "description": "temporary timeseries error"
                    }
                  }
                }"#,
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
        ._preauth("cookie", "crumb")
        .cache_ttl(Duration::from_secs(61))
        .retry_enabled(false)
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, sym);
    let err = ticker.shares().await.unwrap_err();

    api_error.assert();
    assert!(
        matches!(err, YfError::Api(ref message) if message.contains("temporary timeseries error")),
        "expected Yahoo API error, got {err:?}"
    );

    api_error.delete();
    let ok = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param("type", "annualOrdinarySharesNumber")
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"timeseries":{"result":[]}}"#);
    });

    let shares = ticker.shares().await.unwrap();

    ok.assert();
    assert!(shares.is_empty());
}

#[tokio::test]
async fn fundamentals_timeseries_yahoo_error_returns_api_error() {
    let server = MockServer::start();
    let sym = "NOFUNDS";

    let api_error = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param(
                "type",
                "annualTotalRevenue,annualGrossProfit,annualOperatingIncome,annualNetIncome,annualInterestExpense,annualTaxProvision,annualDepreciationAndAmortization",
            )
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "timeseries": {
                    "result": null,
                    "error": {
                      "code": "Not Found",
                      "description": "No fundamentals data found"
                    }
                  }
                }"#,
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
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let err = Ticker::new(&client, sym)
        .income_stmt(None)
        .await
        .unwrap_err();

    api_error.assert();
    match err {
        YfError::Api(message) => assert!(
            message.contains("yahoo error:") && message.contains("No fundamentals data found"),
            "expected Yahoo API error to be surfaced; got {message}"
        ),
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn fundamentals_timeseries_finance_error_returns_api_error() {
    let server = MockServer::start();
    let sym = "BADREQ";

    let api_error = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param("type", "annualOrdinarySharesNumber")
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "finance": {
                    "result": null,
                    "error": {
                      "code": "Bad Request",
                      "description": "Invalid fundamentals timeseries request"
                    }
                  }
                }"#,
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
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let err = Ticker::new(&client, sym).shares().await.unwrap_err();

    api_error.assert();
    match err {
        YfError::Api(message) => assert!(
            message.contains("yahoo error:")
                && message.contains("Invalid fundamentals timeseries request"),
            "expected Yahoo API error to be surfaced; got {message}"
        ),
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn fundamentals_timeseries_missing_result_is_missing_data() {
    let server = MockServer::start();
    let sym = "MALFORMED";

    let malformed = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param("type", "annualOrdinarySharesNumber")
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"timeseries":{"error":null}}"#);
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

    let err = Ticker::new(&client, sym).shares().await.unwrap_err();

    malformed.assert();
    match err {
        YfError::MissingData(message) => assert!(
            message.contains("missing timeseries result"),
            "expected missing timeseries result error; got {message}"
        ),
        other => panic!("expected MissingData error, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_timeseries_value_is_reported_without_dropping_valid_periods() {
    let server = MockServer::start();
    let sym = "BADVALUE";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param(
                "type",
                "annualTotalRevenue,annualGrossProfit,annualOperatingIncome,annualNetIncome,annualInterestExpense,annualTaxProvision,annualDepreciationAndAmortization",
            )
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "timeseries": {
                    "result": [{
                      "timestamp": [1704067200, 1735689600],
                      "meta": {},
                      "annualTotalRevenue": [
                        {
                          "currencyCode": "USD",
                          "reportedValue": { "raw": "not-a-number" }
                        },
                        {
                          "currencyCode": "USD",
                          "reportedValue": { "raw": 42 }
                        }
                      ]
                    }]
                  }
                }"#,
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
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .income_statement_with_diagnostics(false, None)
        .await
        .unwrap();

    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].period.to_string(), "2025-01-01");
    assert_eq!(
        response.data[0]
            .total_revenue
            .as_ref()
            .map(paft::money::Money::currency),
        Some(&Currency::Iso(IsoCurrency::USD))
    );
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "timeseries.reportedValue",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "reportedValue",
                ..
            },
            ..
        } if key == "TotalRevenue@1704067200"
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
async fn malformed_calendar_dates_are_reported_as_projection_loss() {
    let server = MockServer::start();
    let sym = "BADCAL";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "calendarEvents")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "calendarEvents": {
                        "earnings": {
                          "earningsDate": [
                            { "raw": 1704067200 },
                            { "raw": 9223372036854775807 }
                          ]
                        },
                        "exDividendDate": { "raw": 9223372036854775807 },
                        "dividendDate": { "raw": 1704153600 }
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .calendar_with_diagnostics()
        .await
        .unwrap();

    mock.assert();
    assert_eq!(response.data.earnings_dates.len(), 1);
    assert!(response.data.ex_dividend_date.is_none());
    assert!(response.data.dividend_payment_date.is_some());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::DroppedItem {
            item: "calendar_earnings_date",
            reason: ProjectionIssue::InvalidField {
                field: "earningsDate",
                ..
            },
            ..
        }
    )));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "calendarEvents.exDividendDate",
            reason: ProjectionIssue::InvalidField {
                field: "exDividendDate",
                ..
            },
            ..
        }
    )));
}

#[tokio::test]
async fn calendar_wrong_type_optional_date_is_diagnostic_not_json() {
    let server = MockServer::start();
    let sym = "BADCALTYPE";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "calendarEvents")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "calendarEvents": {
                        "earnings": {
                          "earningsDate": [{ "raw": 1704153600 }]
                        },
                        "exDividendDate": "not-a-date",
                        "dividendDate": { "raw": 1704153600 }
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .calendar_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.earnings_dates.len(), 1);
    assert!(response.data.ex_dividend_date.is_none());
    assert!(response.data.dividend_payment_date.is_some());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "calendarEvents.exDividendDate",
            reason: ProjectionIssue::InvalidField {
                field: "exDividendDate",
                ..
            },
            ..
        }
    )));

    let err = FundamentalsBuilder::new(&client, sym)
        .strict()
        .calendar()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn strict_calendar_errors_on_malformed_date() {
    let server = MockServer::start();
    let sym = "BADCAL";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "calendarEvents")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "calendarEvents": {
                        "earnings": {
                          "earningsDate": [{ "raw": 9223372036854775807 }]
                        }
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let err = FundamentalsBuilder::new(&client, sym)
        .strict()
        .calendar()
        .await
        .unwrap_err();

    mock.assert();
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn malformed_share_timestamps_are_reported_as_projection_loss() {
    let server = MockServer::start();
    let sym = "BADSHARES";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param("type", "annualOrdinarySharesNumber")
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "timeseries": {
                    "result": [{
                      "meta": {},
                      "timestamp": [9223372036854775807],
                      "annualOrdinarySharesNumber": [{
                        "reportedValue": { "raw": 100 }
                      }]
                    }]
                  }
                }"#,
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
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .shares_with_diagnostics(false)
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::DroppedItem {
            item: "share_count",
            reason: ProjectionIssue::InvalidField {
                field: "timestamp",
                ..
            },
            ..
        }
    )));

    let err = FundamentalsBuilder::new(&client, sym)
        .strict()
        .shares(false)
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn malformed_share_values_do_not_drop_valid_siblings() {
    let server = MockServer::start();
    let sym = "BADSHAREVALUE";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{sym}"
            ))
            .query_param("symbol", sym)
            .query_param("type", "annualOrdinarySharesNumber")
            .query_param("crumb", "crumb")
            .query_param_exists("period1")
            .query_param_exists("period2");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "timeseries": {
                    "result": [{
                      "meta": {},
                      "timestamp": [1704067200, 1735689600],
                      "annualOrdinarySharesNumber": [
                        { "reportedValue": { "raw": "not-a-number" } },
                        { "reportedValue": { "raw": 100 } }
                      ]
                    }]
                  }
                }"#,
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
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = FundamentalsBuilder::new(&client, sym)
        .shares_with_diagnostics(false)
        .await
        .unwrap();

    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].shares, 100);
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            endpoint: "shares",
            path: "timeseries.reportedValue",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "reportedValue",
                ..
            },
        } if key == "annualOrdinarySharesNumber@1704067200"
    )));

    let err = FundamentalsBuilder::new(&client, sym)
        .strict()
        .shares(false)
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}
