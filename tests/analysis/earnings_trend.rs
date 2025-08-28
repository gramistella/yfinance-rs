use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient};

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

#[tokio::test]
async fn offline_earnings_trend_uses_recorded_fixture() {
    let sym = "AAPL";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", sym))
            .query_param("modules", "earningsTrend")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("analysis_api_earningsTrend", sym));
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
    let rows = t.earnings_trend().await.unwrap();

    mock.assert();
    assert_eq!(rows.len(), 4, "record with YF_RECORD=1 first");

    let current_year = rows.iter().find(|r| r.period == "0y").unwrap();
    assert!(current_year.earnings_estimate_avg.is_some());
    assert!(current_year.revenue_estimate_avg.is_some());
    assert!(current_year.eps_trend_current.is_some());
    assert!(current_year.eps_revisions_up_last_30_days.is_some());
}
