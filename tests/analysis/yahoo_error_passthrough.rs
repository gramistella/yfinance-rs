use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient, YfError};

#[tokio::test]
async fn analysis_other_yahoo_errors_are_surfaced_without_retry() {
    let server = MockServer::start();
    let sym = "AAPL";

    // Simulate a non-crumb Yahoo error response
    let api_err = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "recommendationTrend")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"quoteSummary":{"result":null,"error":{"description":"Something broke"}}}"#);
    });

    // Build a client that already has credentials so the call proceeds immediately.
    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(client, sym);
    let err = t.recommendations().await.unwrap_err();

    api_err.assert();

    match err {
        YfError::Data(s) => assert!(
            s.to_ascii_lowercase().contains("yahoo error:") && s.contains("Something broke"),
            "expected yahoo error to be surfaced; got {s}"
        ),
        other => panic!("expected Data error, got {other:?}"),
    }
}
