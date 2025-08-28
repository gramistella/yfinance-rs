use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{Range, Ticker, YfClient};

#[tokio::test]
async fn offline_capital_gains_from_history() {
    let server = MockServer::start();
    let sym = "VFINX";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v8/finance/chart/{}", sym))
            .query_param("range", "max")
            .query_param("interval", "1d")
            .query_param("events", "div|split|capitalGains");
        then.status(200)
            .header("content-type", "application/json")
            .body(crate::common::fixture("history_chart", sym, "json"));
    });

    let client = YfClient::builder()
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let t = Ticker::new(client, sym);
    let gains = t.capital_gains(Some(Range::Max)).await.unwrap();

    mock.assert();
    assert!(
        !gains.is_empty(),
        "capital gains missing from fixture for VFINX. Did you run `just test-record ticker`?"
    );
    assert!(gains[0].1 > 0.0, "gain amount should be positive");
}
