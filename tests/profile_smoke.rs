use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{Profile, YfClient};

fn html_company() -> String {
    format!(
        r#"<html><script>root.App.main = {{ "context": {{ "dispatcher": {{ "stores": {{ "QuoteSummaryStore": {{
        "quoteType": {{ "quoteType": "EQUITY", "longName": "Apple Inc.", "shortName": "Apple" }},
        "summaryProfile": {{
            "address1": "One Apple Park Way",
            "city": "Cupertino",
            "state": "CA",
            "country": "United States",
            "zip": "95014",
            "sector": "Technology",
            "industry": "Consumer Electronics",
            "longBusinessSummary": "…",
            "website": "https://www.apple.com"
        }}
    }}}} }} }} }}; </script></html>"#
    )
}

fn html_fund() -> String {
    r#"<html><script>root.App.main = { "context": { "dispatcher": { "stores": { "QuoteSummaryStore": {
        "quoteType": { "quoteType": "ETF", "longName": "Invesco QQQ Trust" },
        "fundProfile": { "legalType": "Exchange Traded Fund", "family": "Invesco" }
    }}}}}; </script></html>"#.to_string()
}

#[tokio::test]
async fn profile_company_happy() {
    let server = MockServer::start();
    let sym = "AAPL";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/{}", sym))
            .query_param("p", sym);
        then.status(200)
            .header("content-type", "text/html")
            .body(html_company());
    });

    let mut client = YfClient::builder()
        .base_quote(Url::parse(&format!("{}/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let prof = Profile::load(&mut client, sym).await.unwrap();
    mock.assert();

    match prof {
        Profile::Company(c) => {
            assert_eq!(c.name, "Apple Inc.");
            assert_eq!(c.sector.as_deref(), Some("Technology"));
            assert_eq!(c.industry.as_deref(), Some("Consumer Electronics"));
            assert_eq!(c.website.as_deref(), Some("https://www.apple.com"));
            assert!(c.address.is_some());
        }
        _ => panic!("expected Company"),
    }
}

#[tokio::test]
async fn profile_fund_happy() {
    let server = MockServer::start();
    let sym = "QQQ";

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/{}", sym))
            .query_param("p", sym);
        then.status(200)
            .header("content-type", "text/html")
            .body(html_fund());
    });

    let mut client = YfClient::builder()
        .base_quote(Url::parse(&format!("{}/", server.base_url())).unwrap())
        .build()
        .unwrap();

    let prof = Profile::load(&mut client, sym).await.unwrap();
    mock.assert();

    match prof {
        Profile::Fund(f) => {
            assert_eq!(f.name, "Invesco QQQ Trust");
            assert_eq!(f.family.as_deref(), Some("Invesco"));
            assert_eq!(f.kind, "Exchange Traded Fund");
        }
        _ => panic!("expected Fund"),
    }
}
