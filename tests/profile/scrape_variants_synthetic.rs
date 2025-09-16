use httpmock::{Method::GET, MockServer};
use paft::fundamentals::Profile;
use url::Url;
use yfinance_rs::{ApiPreference, YfClient};

fn svelte_html(payload: &str) -> String {
    format!(
        r#"<!doctype html>
<html><body>
<script type="application/json" data-sveltekit-fetched="1">{payload}</script>
</body></html>"#
    )
}

#[tokio::test]
async fn scrape_sveltekit_equity() {
    let server = MockServer::start();
    let sym = "DEMO";

    // data-sveltekit-fetched array → nodes[].data.quoteSummary.result[0]
    let payload = r#"[{"nodes":[{"data":{"quoteSummary":{"result":[{
        "quoteType":{"quoteType":"EQUITY","longName":"Demo Co"},
        "summaryProfile":{"sector":"Tech","industry":"Gadgets","website":"https://demo.invalid"}
    }]}}}]}]"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/quote/{sym}"))
            .query_param("p", sym);
        then.status(200)
            .header("content-type", "text/html")
            .body(svelte_html(payload));
    });

    let client = YfClient::builder()
        .base_quote(Url::parse(&format!("{}/quote/", server.base_url())).unwrap())
        ._api_preference(ApiPreference::ScrapeOnly)
        .build()
        .unwrap();

    let prof = yfinance_rs::profile::load_profile(&client, sym)
        .await
        .unwrap();
    mock.assert();

    match prof {
        Profile::Company(c) => {
            assert_eq!(c.name, "Demo Co");
            assert_eq!(c.sector.as_deref(), Some("Tech"));
        }
        _ => panic!("expected Company"),
    }
}

#[tokio::test]
async fn scrape_infers_equity_when_quote_type_missing() {
    let server = MockServer::start();
    let sym = "INFER";

    // No quoteType; presence of summaryProfile should infer EQUITY
    let payload = r#"[{"nodes":[{"data":{"quoteSummary":{"result":[{
        "summaryProfile":{"sector":"Tech"}
    }]}}}]}]"#;

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/quote/{sym}"))
            .query_param("p", sym);
        then.status(200)
            .header("content-type", "text/html")
            .body(svelte_html(payload));
    });

    let client = YfClient::builder()
        .base_quote(Url::parse(&format!("{}/quote/", server.base_url())).unwrap())
        ._api_preference(ApiPreference::ScrapeOnly)
        .build()
        .unwrap();

    let prof = yfinance_rs::profile::load_profile(&client, sym)
        .await
        .unwrap();
    mock.assert();
    assert!(matches!(prof, Profile::Company(_)));
}
