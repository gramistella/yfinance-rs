use httpmock::{Method::GET, MockServer};
use paft::money::{Currency, IsoCurrency};
use url::Url;
use yfinance_rs::{ProjectionIssue, YfClient, YfError, YfWarning, analysis::AnalysisBuilder};

#[tokio::test]
async fn missing_recommendation_trend_module_is_provider_unavailable() {
    let sym = "NORECS";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "recommendationTrend")
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

    let response = AnalysisBuilder::new(&client, sym)
        .recommendations_with_diagnostics()
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::ProviderFeatureUnavailable {
            feature: "recommendationTrend",
            reason: ProjectionIssue::ProviderUnavailable {
                feature: "recommendationTrend"
            },
            ..
        })
    ));

    let err = AnalysisBuilder::new(&client, sym)
        .strict()
        .recommendations()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn missing_earnings_trend_module_is_provider_unavailable() {
    let sym = "NOTREND";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earningsTrend")
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

    let response = AnalysisBuilder::new(&client, sym)
        .earnings_trend_with_diagnostics(None)
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::ProviderFeatureUnavailable {
            feature: "earningsTrend",
            reason: ProjectionIssue::ProviderUnavailable {
                feature: "earningsTrend"
            },
            ..
        })
    ));

    let err = AnalysisBuilder::new(&client, sym)
        .strict()
        .earnings_trend(None)
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn missing_price_target_module_is_provider_unavailable() {
    let sym = "NOPRICE";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "financialData")
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

    let response = AnalysisBuilder::new(&client, sym)
        .analyst_price_target_with_diagnostics(None)
        .await
        .unwrap();

    assert_eq!(response.data.mean, None);
    assert_eq!(response.data.number_of_analysts, None);
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::ProviderFeatureUnavailable {
            feature: "financialData",
            reason: ProjectionIssue::ProviderUnavailable {
                feature: "financialData"
            },
            ..
        })
    ));

    let err = AnalysisBuilder::new(&client, sym)
        .strict()
        .analyst_price_target(None)
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn recommendation_trend_missing_period_reports_dropped_row() {
    let sym = "AAPL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "recommendationTrend")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "recommendationTrend": {
                        "trend": [{
                          "strongBuy": 1,
                          "buy": 2,
                          "hold": 3,
                          "sell": 4,
                          "strongSell": 5
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

    let response = AnalysisBuilder::new(&client, sym)
        .recommendations_with_diagnostics()
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::DroppedItem {
            item: "recommendation_trend",
            reason: ProjectionIssue::MissingRequiredField { field: "period" },
            ..
        })
    ));

    let err = AnalysisBuilder::new(&client, sym)
        .strict()
        .recommendations()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn recommendation_counts_report_invalid_present_values() {
    let sym = "AAPL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "recommendationTrend")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "recommendationTrend": {
                        "trend": [{
                          "period": "0m",
                          "strongBuy": -1,
                          "buy": 2,
                          "hold": 3,
                          "sell": 4,
                          "strongSell": 5
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

    let response = AnalysisBuilder::new(&client, sym)
        .recommendations_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].strong_buy, None);
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "recommendationTrend.trend[].strongBuy",
            reason: ProjectionIssue::InvalidField {
                field: "strongBuy",
                ..
            },
            ..
        }
    )));

    let err = AnalysisBuilder::new(&client, sym)
        .strict()
        .recommendations()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn recommendation_counts_wrong_type_is_reported_as_projection_loss() {
    let sym = "AAPL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "recommendationTrend")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "recommendationTrend": {
                        "trend": [{
                          "period": "0m",
                          "strongBuy": "one",
                          "buy": 2,
                          "hold": 3,
                          "sell": 4,
                          "strongSell": 5
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

    let response = AnalysisBuilder::new(&client, sym)
        .recommendations_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].strong_buy, None);
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "recommendationTrend.trend[].strongBuy",
            reason: ProjectionIssue::InvalidField {
                field: "strongBuy",
                ..
            },
            ..
        }
    )));

    let err = AnalysisBuilder::new(&client, sym)
        .strict()
        .recommendations()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn upgrades_downgrades_keep_rows_when_optional_fields_are_invalid() {
    let sym = "AAPL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "upgradeDowngradeHistory")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "upgradeDowngradeHistory": {
                        "history": [
                          {
                            "epochGradeDate": 1704067200,
                            "firm": "Example Research",
                            "fromGrade": "!!!",
                            "toGrade": "Buy",
                            "action": "!!!"
                          },
                          {
                            "epochGradeDate": 1704153600,
                            "firm": [],
                            "fromGrade": "Hold",
                            "toGrade": "Buy",
                            "action": "up"
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

    let response = AnalysisBuilder::new(&client, sym)
        .upgrades_downgrades_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.len(), 2);
    assert!(response.data[0].from_grade.is_none());
    assert_eq!(
        response.data[0]
            .to_grade
            .as_ref()
            .map(std::string::ToString::to_string)
            .as_deref(),
        Some("BUY")
    );
    assert!(response.data[0].action.is_none());
    assert!(response.data[1].firm.is_none());
    for path in [
        "upgradeDowngradeHistory.history[].fromGrade",
        "upgradeDowngradeHistory.history[].action",
    ] {
        assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
            warning,
            YfWarning::OmittedPresentField {
                path: warning_path,
                key: Some(key),
                reason: ProjectionIssue::InvalidField { .. },
                ..
            } if *warning_path == path && key == "Example Research"
        )));
    }
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "upgradeDowngradeHistory.history[].firm",
            key: None,
            reason: ProjectionIssue::InvalidField { field: "firm", .. },
            ..
        }
    )));

    let err = AnalysisBuilder::new(&client, sym)
        .strict()
        .upgrades_downgrades()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn price_target_reports_present_prices_when_currency_cannot_be_resolved() {
    let sym = "NOCURRENCY";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "financialData")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "financialData": {
                        "targetMeanPrice": { "raw": 10.0 },
                        "targetHighPrice": { "raw": 12.0 },
                        "targetLowPrice": { "raw": 8.0 },
                        "numberOfAnalystOpinions": { "raw": 7 }
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

    let response = AnalysisBuilder::new(&client, sym)
        .analyst_price_target_with_diagnostics(None)
        .await
        .unwrap();

    mock.assert();
    assert!(response.data.mean.is_none());
    assert_eq!(response.data.number_of_analysts, Some(7));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "financialData.targetMeanPrice",
            reason: ProjectionIssue::CurrencyUnresolved,
            ..
        }
    )));
}

#[tokio::test]
async fn price_target_uses_quote_currency_enrichment_in_strict_mode() {
    let sym = "PRICEFX";
    let server = MockServer::start();

    let target_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "financialData")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "financialData": {
                        "targetMeanPrice": { "raw": 10.0 },
                        "targetHighPrice": { "raw": 12.0 },
                        "targetLowPrice": { "raw": 8.0 }
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
                r#"{
                  "quoteResponse": {
                    "result": [{
                      "symbol": "PRICEFX",
                      "quoteType": "EQUITY",
                      "currency": "USD"
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

    let response = AnalysisBuilder::new(&client, sym)
        .strict()
        .analyst_price_target_with_diagnostics(None)
        .await
        .unwrap();

    target_mock.assert();
    quote_mock.assert();
    assert!(response.diagnostics.is_empty());
    assert_eq!(
        response
            .data
            .mean
            .as_ref()
            .map(|price| price.currency().clone()),
        Some(Currency::Iso(IsoCurrency::USD))
    );
}

#[tokio::test]
async fn earnings_trend_omits_values_when_enriched_currency_is_invalid() {
    let sym = "BADENRICH";
    let server = MockServer::start();

    let trend_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "earningsTrend")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "earningsTrend": {
                        "trend": [{
                          "period": "0q",
                          "revenueEstimate": {
                            "avg": { "raw": 1000 },
                            "numberOfAnalysts": { "raw": 3 }
                          }
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
                r#"{"quoteResponse":{"result":[{"symbol":"BADENRICH","quoteType":"EQUITY","financialCurrency":"!!!"}],"error":null}}"#,
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

    let response = AnalysisBuilder::new(&client, sym)
        .earnings_trend_with_diagnostics(None)
        .await
        .unwrap();

    trend_mock.assert();
    quote_mock.assert();
    assert_eq!(response.data.len(), 1);
    assert!(response.data[0].revenue_estimate.avg.is_none());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "earningsTrend[].revenueEstimate.avg",
            reason: ProjectionIssue::InvalidCurrency { code },
            ..
        } if code == "!!!"
    )));
}

#[tokio::test]
async fn price_target_accepts_override_currency_in_strict_mode() {
    let sym = "OVERRIDE";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "financialData")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "financialData": {
                        "targetMeanPrice": { "raw": 10.0 }
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

    let response = AnalysisBuilder::new(&client, sym)
        .strict()
        .analyst_price_target_with_diagnostics(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();

    mock.assert();
    assert!(response.data.mean.is_some());
    assert!(response.diagnostics.is_empty());
}

#[tokio::test]
async fn fractional_analyst_count_is_rejected_without_rounding() {
    let sym = "MSFT";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "financialData")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "financialData": {
                        "numberOfAnalystOpinions": { "raw": 12.7 }
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

    let response = AnalysisBuilder::new(&client, sym)
        .analyst_price_target_with_diagnostics(None)
        .await
        .unwrap();

    assert_eq!(response.data.number_of_analysts, None);
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "financialData.numberOfAnalystOpinions",
            reason: ProjectionIssue::InvalidField {
                field: "numberOfAnalystOpinions",
                ..
            },
            ..
        }
    )));

    let err = AnalysisBuilder::new(&client, sym)
        .strict()
        .analyst_price_target(None)
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}
