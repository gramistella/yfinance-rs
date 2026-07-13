use httpmock::{Method::GET, MockServer};
use paft::Decimal;
use paft::fundamentals::holders::TransactionType;
use url::Url;
use yfinance_rs::{HoldersBuilder, ProjectionIssue, Ticker, YfClient, YfError, YfWarning};

const INSTITUTION_OWNERSHIP: &str = "institutionOwnership";
const MAJOR_HOLDERS: &str = "majorHoldersBreakdown";
const INSIDER_HOLDERS: &str = "insiderHolders";
const NET_SHARE_PURCHASE_ACTIVITY: &str = "netSharePurchaseActivity";

#[tokio::test]
async fn missing_institution_ownership_module_is_provider_unavailable() {
    let sym = "NOOWN";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INSTITUTION_OWNERSHIP)
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

    let response = HoldersBuilder::new(&client, sym)
        .institutional_holders_with_diagnostics()
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::ProviderFeatureUnavailable {
            feature: "institutionOwnership",
            reason: ProjectionIssue::ProviderUnavailable {
                feature: "institutionOwnership"
            },
            ..
        })
    ));

    let err = HoldersBuilder::new(&client, sym)
        .strict()
        .institutional_holders()
        .await
        .unwrap_err();

    holders_mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn missing_ownership_list_is_provider_unavailable() {
    let sym = "NOLIST";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INSTITUTION_OWNERSHIP)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{ "institutionOwnership": {} }],
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

    let response = HoldersBuilder::new(&client, sym)
        .institutional_holders_with_diagnostics()
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::ProviderFeatureUnavailable {
            feature: "institutionOwnership.ownershipList",
            reason: ProjectionIssue::ProviderUnavailable {
                feature: "institutionOwnership.ownershipList"
            },
            ..
        })
    ));

    let err = HoldersBuilder::new(&client, sym)
        .strict()
        .institutional_holders()
        .await
        .unwrap_err();

    holders_mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn malformed_optional_holder_raw_field_is_omitted_without_dropping_row() {
    let sym = "BADHOLDER";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INSTITUTION_OWNERSHIP)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "institutionOwnership": {
                        "ownershipList": [
                          {
                            "organization": "Malformed Capital",
                            "position": { "raw": "not-a-number" },
                            "reportDate": { "raw": 1704067200 }
                          },
                          {
                            "organization": "Valid Capital",
                            "position": { "raw": 10 },
                            "reportDate": { "raw": 1704067200 }
                          }
                        ]
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

    let response = HoldersBuilder::new(&client, sym)
        .institutional_holders_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.len(), 2);
    assert_eq!(response.data[0].holder, "Malformed Capital");
    assert!(response.data[0].shares.is_none());
    assert_eq!(response.data[1].holder, "Valid Capital");
    assert_eq!(response.data[1].shares, Some(10));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            endpoint: "holders",
            path: "ownershipList[].position",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "position",
                ..
            },
        } if key == "Malformed Capital"
    )));

    let err = HoldersBuilder::new(&client, sym)
        .strict()
        .institutional_holders()
        .await
        .unwrap_err();

    holders_mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn insider_roster_missing_position_is_not_defaulted_to_officer() {
    let sym = "AAPL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INSIDER_HOLDERS)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "insiderHolders": {
                        "holders": [{
                          "name": "MISSING POSITION",
                          "transactionDescription": "Sale",
                          "latestTransDate": { "raw": 1704067200 },
                          "positionDirectDate": { "raw": 1704067200 }
                        }]
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

    let rows = Ticker::new(&client, sym)
        .insider_roster_holders()
        .await
        .unwrap();

    mock.assert();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn optional_holder_value_is_omitted_when_currency_cannot_be_resolved() {
    let sym = "NOCURRENCY";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INSTITUTION_OWNERSHIP)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "institutionOwnership": {
                        "ownershipList": [{
                          "organization": "No Currency Capital",
                          "position": { "raw": 10 },
                          "reportDate": { "raw": 1704067200 },
                          "value": { "raw": 12345 }
                        }]
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let rows = Ticker::new(&client, sym)
        .institutional_holders()
        .await
        .unwrap();

    holders_mock.assert();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].holder, "No Currency Capital");
    assert_eq!(rows[0].shares, Some(10));
    assert!(rows[0].value.is_none());
}

#[tokio::test]
async fn holder_diagnostics_report_present_value_with_unresolved_currency() {
    let sym = "NOCURRENCY";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INSTITUTION_OWNERSHIP)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "institutionOwnership": {
                        "ownershipList": [{
                          "organization": "No Currency Capital",
                          "position": { "raw": 10 },
                          "reportDate": { "raw": 1704067200 },
                          "value": { "raw": 12345 }
                        }]
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let response = HoldersBuilder::new(&client, sym)
        .institutional_holders_with_diagnostics()
        .await
        .unwrap();

    holders_mock.assert();
    assert_eq!(response.data.len(), 1);
    assert!(response.data[0].value.is_none());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::OmittedPresentField {
            path: "ownershipList[].value",
            reason: ProjectionIssue::CurrencyUnresolved,
            ..
        })
    ));
}

#[tokio::test]
async fn strict_holder_value_rejects_invalid_enriched_currency_as_data_quality() {
    let sym = "BADENRICH";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", INSTITUTION_OWNERSHIP)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "institutionOwnership": {
                        "ownershipList": [{
                          "organization": "Invalid Currency Capital",
                          "position": { "raw": 10 },
                          "reportDate": { "raw": 1704067200 },
                          "value": { "raw": 12345 }
                        }]
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{"quoteResponse":{"result":[{"symbol":"BADENRICH","quoteType":"EQUITY","currency":"!!!"}],"error":null}}"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let err = HoldersBuilder::new(&client, sym)
        .strict()
        .institutional_holders_with_diagnostics()
        .await
        .unwrap_err();

    holders_mock.assert();
    quote_mock.assert();
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn insider_transaction_value_uses_trading_currency_when_financial_currency_differs() {
    let sym = "SAP";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "insiderTransactions")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "insiderTransactions": {
                        "transactions": [{
                          "filerName": "EXAMPLE INSIDER",
                          "filerRelation": "Officer",
                          "transactionText": "Sale",
                          "shares": { "raw": 10 },
                          "value": { "raw": 1234 },
                          "startDate": { "raw": 1704067200 },
                          "filerUrl": ""
                        }]
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });
    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", sym);
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{"quoteResponse":{"result":[{"symbol":"SAP","quoteType":"EQUITY","currency":"USD","financialCurrency":"EUR"}],"error":null}}"#,
            );
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let rows = Ticker::new(&client, sym)
        .insider_transactions()
        .await
        .unwrap();

    holders_mock.assert();
    quote_mock.assert();
    assert_eq!(rows.len(), 1);
    let value = rows[0].value.as_ref().expect("value should map");
    assert_eq!(value.currency().to_string(), "USD");
    assert_eq!(value.amount(), paft::Decimal::from(1234u64));
    assert!(rows[0].url.is_none());
}

#[tokio::test]
async fn blank_no_cash_insider_transaction_is_inferred_as_exercise() {
    let sym = "AAPL";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "insiderTransactions")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "insiderTransactions": {
                        "transactions": [{
                          "filerName": "COOK TIMOTHY D",
                          "filerRelation": "Officer",
                          "transactionText": "",
                          "shares": { "raw": 131576 },
                          "value": null,
                          "startDate": { "raw": 1775001600 },
                          "filerUrl": ""
                        }]
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

    let response = HoldersBuilder::new(&client, sym)
        .insider_transactions_with_diagnostics()
        .await
        .unwrap();

    holders_mock.assert();
    assert!(response.diagnostics.is_empty());
    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].transaction_type, TransactionType::Exercise);
    assert_eq!(response.data[0].shares, Some(131_576));
    assert!(response.data[0].value.is_none());
    assert!(response.data[0].url.is_none());
}

#[tokio::test]
async fn major_holder_preserves_exact_ratio_and_reports_unrepresentable_decimal() {
    let sym = "MAJOR";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", MAJOR_HOLDERS)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "majorHoldersBreakdown": {
                        "insidersPercentHeld": { "raw": 1e30 },
                        "institutionsPercentHeld": { "raw": 0.1234567890123456789012345678 },
                        "institutionsCount": { "raw": 42 }
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

    let response = HoldersBuilder::new(&client, sym)
        .major_holders_with_diagnostics()
        .await
        .unwrap();

    holders_mock.assert();
    assert_eq!(response.data.len(), 1);
    assert!(
        response
            .data
            .iter()
            .any(|holder| { holder.category.contains("% of Shares Held by Institutions") })
    );
    assert_eq!(
        response.data[0].value.as_decimal(),
        &"0.1234567890123456789012345678".parse::<Decimal>().unwrap()
    );
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "majorHoldersBreakdown.insidersPercentHeld",
            reason: ProjectionIssue::InvalidField {
                field: "insidersPercentHeld",
                ..
            },
            ..
        }
    )));
}

#[tokio::test]
async fn net_share_purchase_activity_missing_period_is_reported() {
    let sym = "NETBAD";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", NET_SHARE_PURCHASE_ACTIVITY)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "netSharePurchaseActivity": {
                        "period": "",
                        "buyInfoShares": { "raw": 10 }
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

    let response = HoldersBuilder::new(&client, sym)
        .net_share_purchase_activity_with_diagnostics()
        .await
        .unwrap();

    holders_mock.assert();
    assert!(response.data.is_none());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::DroppedItem {
            item: "net_share_purchase_activity",
            reason: ProjectionIssue::MissingRequiredField { field: "period" },
            ..
        })
    ));
}

#[tokio::test]
async fn strict_net_share_purchase_activity_errors_on_missing_period() {
    let sym = "NETBAD";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", NET_SHARE_PURCHASE_ACTIVITY)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "netSharePurchaseActivity": { "period": "" }
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

    let err = HoldersBuilder::new(&client, sym)
        .strict()
        .net_share_purchase_activity()
        .await
        .unwrap_err();

    holders_mock.assert();
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn net_share_purchase_activity_unrepresentable_percent_is_reported() {
    let sym = "NETPCT";
    let server = MockServer::start();

    let holders_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", NET_SHARE_PURCHASE_ACTIVITY)
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "netSharePurchaseActivity": {
                        "period": "6m",
                        "buyInfoShares": { "raw": 10 },
                        "netPercentInsiderShares": { "raw": 1e30 }
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

    let response = HoldersBuilder::new(&client, sym)
        .net_share_purchase_activity_with_diagnostics()
        .await
        .unwrap();

    holders_mock.assert();
    let activity = response
        .data
        .expect("valid period should keep the activity");
    assert_eq!(activity.buy_shares, Some(10));
    assert!(activity.net_percent_insider_shares.is_none());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::OmittedPresentField {
            path: "netSharePurchaseActivity.netPercentInsiderShares",
            reason: ProjectionIssue::InvalidField {
                field: "netPercentInsiderShares",
                ..
            },
            ..
        })
    ));
}
