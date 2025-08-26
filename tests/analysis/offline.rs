use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient};

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

#[tokio::test]
async fn offline_recommendations_trend_uses_recorded_fixture() {
    let sym = "AAPL";
    let server = MockServer::start();

    // Trend only
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", sym))
            .query_param("modules", "recommendationTrend")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("analysis_api", sym));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(client, sym).unwrap();
    let rows = t.recommendations().await.unwrap();

    mock.assert();
    assert!(!rows.is_empty(), "record with YF_RECORD=1 first");
}

#[tokio::test]
async fn offline_recommendations_summary_uses_recorded_fixture() {
    let sym = "MSFT";
    let server = MockServer::start();

    // Combined modules (trend + mean)
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", sym))
            .query_param("modules", "recommendationTrend,recommendationMean")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("analysis_api", sym));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(client, sym).unwrap();
    let s = t.recommendations_summary().await.unwrap();

    mock.assert();
    assert!(
        s.strong_buy + s.buy + s.hold + s.sell + s.strong_sell > 0,
        "record with YF_RECORD=1 first"
    );
}

#[tokio::test]
async fn offline_upgrades_downgrades_uses_recorded_fixture() {
    let sym = "GOOGL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", sym))
            .query_param("modules", "upgradeDowngradeHistory")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("analysis_api", sym));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .api_preference(ApiPreference::ApiOnly)
        .preauth("cookie", "crumb")
        .build()
        .unwrap();

    let t = Ticker::new(client, sym).unwrap();
    let _rows = t.upgrades_downgrades().await.unwrap();

    mock.assert();
    // After recording, this should be non-empty for many large-caps — but allow empty in case the day is quiet.
}
