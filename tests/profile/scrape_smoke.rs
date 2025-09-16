use crate::common::{mock_profile_scrape, setup_server};
use paft::fundamentals::Profile;
use url::Url;
use yfinance_rs::{ApiPreference, YfClient};

#[tokio::test]
async fn profile_scrape_company_happy() {
    let server = setup_server();
    let sym = "AAPL";
    let mock = mock_profile_scrape(&server, sym);

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
async fn profile_scrape_fund_happy() {
    let server = setup_server();
    let sym = "QQQ";
    let mock = mock_profile_scrape(&server, sym);

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
        Profile::Fund(f) => {
            assert_eq!(f.name, "Invesco QQQ Trust");
            assert_eq!(f.family.as_deref(), Some("Invesco"));
            assert_eq!(f.kind.to_string(), "ETF");
        }
        _ => panic!("expected Fund"),
    }
}
