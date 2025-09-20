use paft::fundamentals::profile::Profile;

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_profile_company() {
    if !(std::env::var("YF_LIVE").ok().as_deref() == Some("1")
        || std::env::var("YF_RECORD").ok().as_deref() == Some("1"))
    {
        return;
    }
    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let prof = yfinance_rs::profile::load_profile(&client, "AAPL")
        .await
        .unwrap();
    match prof {
        Profile::Company(c) => {
            assert!(!c.name.is_empty());
        }
        Profile::Fund(_) => panic!("expected company"),
    }
}

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_profile_fund_for_record() {
    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        return;
    }
    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let _ = yfinance_rs::profile::load_profile(&client, "QQQ").await;
}

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_profile_fund_scrape_for_record() {
    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        return;
    }
    let client = yfinance_rs::YfClient::builder()
        ._api_preference(yfinance_rs::ApiPreference::ScrapeOnly)
        .build()
        .unwrap();
    let _ = yfinance_rs::profile::load_profile(&client, "AAPL").await;
}

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_profile_company_scrape_for_record() {
    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        return;
    }
    let client = yfinance_rs::YfClient::builder()
        ._api_preference(yfinance_rs::ApiPreference::ScrapeOnly)
        .build()
        .unwrap();
    let _ = yfinance_rs::profile::load_profile(&client, "QQQ").await;
}
