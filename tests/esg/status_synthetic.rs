use httpmock::{Method::GET, MockServer};
use paft::Decimal;
use url::Url;
use yfinance_rs::{EsgBuilder, ProjectionIssue, Ticker, YfClient, YfError, YfWarning};

fn preauthed_client(server: &MockServer) -> YfClient {
    YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap()
}

const NOT_FOUND_BODY: &str = r#"{
  "quoteSummary": {
    "error": {
      "code": "Not Found",
      "description": "No fundamentals data found for symbol: MSFT"
    },
    "result": null
  }
}"#;

#[tokio::test]
async fn esg_http_not_found_returns_error() {
    let sym = "MSFT";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "esgScores")
            .query_param("crumb", "crumb");
        then.status(404)
            .header("content-type", "application/json")
            .body(NOT_FOUND_BODY);
    });

    let ticker = Ticker::new(&preauthed_client(&server), sym);
    let err = ticker.sustainability().await.unwrap_err();

    mock.assert();
    assert!(err.to_string().contains("Not found"));
}

#[tokio::test]
async fn esg_not_found_body_returns_error() {
    let sym = "MSFT";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "esgScores")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(NOT_FOUND_BODY);
    });

    let ticker = Ticker::new(&preauthed_client(&server), sym);
    let err = ticker.sustainability().await.unwrap_err();

    mock.assert();
    assert!(err.to_string().contains("No fundamentals data found"));
}

#[tokio::test]
async fn missing_esg_module_is_reported_as_unavailable() {
    let sym = "MSFT";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "esgScores")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{}],
                    "error": null
                  }
                }"#,
            );
    });

    let client = preauthed_client(&server);
    let response = EsgBuilder::new(&client, sym)
        .fetch_with_diagnostics()
        .await
        .unwrap();

    mock.assert();
    assert!(response.data.scores.is_none());
    assert!(matches!(
        response.diagnostics.warnings.first(),
        Some(YfWarning::ProviderFeatureUnavailable {
            feature: "esgScores",
            reason: ProjectionIssue::ProviderUnavailable {
                feature: "esgScores"
            },
            ..
        })
    ));

    let err = EsgBuilder::new(&client, sym)
        .strict()
        .fetch()
        .await
        .unwrap_err();
    assert!(matches!(err, YfError::DataQuality(_)));
}

#[tokio::test]
async fn malformed_esg_raw_score_is_omitted_without_json_error() {
    let sym = "BADESG";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "esgScores")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                  "quoteSummary": {
                    "result": [{
                      "esgScores": {
                        "environmentScore": { "raw": "not-a-number" },
                        "socialScore": { "raw": 0.1234567890123456789012345678 },
                        "governanceScore": { "raw": 3.5 }
                      }
                    }],
                    "error": null
                  }
                }"#,
            );
    });

    let client = preauthed_client(&server);
    let response = EsgBuilder::new(&client, sym)
        .fetch_with_diagnostics()
        .await
        .unwrap();

    let scores = response
        .data
        .scores
        .expect("valid siblings keep ESG scores");
    assert!(scores.environmental.is_none());
    assert_eq!(
        scores.social,
        Some("0.1234567890123456789012345678".parse::<Decimal>().unwrap())
    );
    assert!(scores.governance.is_some());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::OmittedPresentField {
            path: "esgScores.environmentScore",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "environmentScore",
                ..
            },
            ..
        } if key == sym
    )));

    let err = EsgBuilder::new(&client, sym)
        .strict()
        .fetch()
        .await
        .unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}
