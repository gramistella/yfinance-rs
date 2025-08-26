use httpmock::Method::GET;
use httpmock::MockServer;
use url::Url;
use yfinance_rs::{ApiPreference, Ticker, YfClient};

#[tokio::test]
async fn upgrades_downgrades_are_sorted_by_ts() {
    let server = MockServer::start();
    let sym = "GOOGL";

    let body = r#"{
      "quoteSummary": {
        "result": [{
          "upgradeDowngradeHistory": {
            "history": [
              { "epochGradeDate": 2000, "firm": "B", "fromGrade": "Hold", "toGrade": "Buy", "action": "up" },
              { "epochGradeDate": 1000, "firm": "A", "fromGrade": "Sell", "toGrade": "Hold", "action": "up" }
            ]
          }
        }],
        "error": null
      }
    }"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", sym))
            .query_param("modules", "upgradeDowngradeHistory")
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

    let t = Ticker::new(client, sym).unwrap();
    let rows = t.upgrades_downgrades().await.unwrap();

    mock.assert();

    assert_eq!(rows.len(), 2);
    assert!(rows[0].ts <= rows[1].ts, "rows should be sorted ascending");
    assert_eq!(rows[0].firm.as_deref(), Some("A"));
    assert_eq!(rows[1].firm.as_deref(), Some("B"));
}
