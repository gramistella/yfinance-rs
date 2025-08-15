use httpmock::{Method::GET, MockServer};
use url::Url;
use yfinance_rs::{Profile, YfClient};

fn api_company_payload() -> String {
    r#"{
      "quoteSummary": {
        "result": [{
          "assetProfile": {
            "address1": "One Apple Park Way",
            "city": "Cupertino",
            "state": "CA",
            "country": "United States",
            "zip": "95014",
            "sector": "Technology",
            "industry": "Consumer Electronics",
            "website": "https://www.apple.com",
            "longBusinessSummary": "..."
          },
          "quoteType": { "quoteType": "EQUITY", "longName": "Apple Inc.", "shortName": "Apple" }
        }],
        "error": null
      }
    }"#
    .to_string()
}

fn api_fund_payload() -> String {
    r#"{
      "quoteSummary": {
        "result": [{
          "fundProfile": { "legalType": "Exchange Traded Fund", "family": "Invesco" },
          "quoteType": { "quoteType": "ETF", "longName": "Invesco QQQ Trust" }
        }],
        "error": null
      }
    }"#
    .to_string()
}

#[tokio::test]
async fn profile_api_company_happy() {
    let server = MockServer::start();
    let sym = "AAPL";
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/{}", sym))
            .query_param("modules", "assetProfile,quoteType,fundProfile");
        then.status(200)
            .header("content-type", "application/json")
            .body(api_company_payload());
    });

    let mut client = YfClient::builder()
        .base_quote_api(Url::parse(&format!("{}/", server.base_url())).unwrap())
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
        }
        _ => panic!("expected Company"),
    }
}

#[tokio::test]
async fn profile_api_fund_happy() {
    let server = MockServer::start();
    let sym = "QQQ";
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/{}", sym))
            .query_param("modules", "assetProfile,quoteType,fundProfile");
        then.status(200)
            .header("content-type", "application/json")
            .body(api_fund_payload());
    });

    let mut client = YfClient::builder()
        .base_quote_api(Url::parse(&format!("{}/", server.base_url())).unwrap())
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
