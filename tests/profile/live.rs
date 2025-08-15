// tests/profile_live.rs
#[tokio::test]
#[ignore]
async fn live_profile_company() {
    if std::env::var("YF_LIVE").ok().as_deref() != Some("1")
        && std::env::var("YF_RECORD").ok().as_deref() != Some("1")
    {
        return;
    }

    let mut client = yfinance_rs::YfClient::builder().build().unwrap();
    let prof = yfinance_rs::Profile::load(&mut client, "AAPL")
        .await
        .unwrap();

    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        match prof {
            yfinance_rs::Profile::Company(c) => assert_eq!(c.name, "Apple Inc."),
            _ => panic!("expected Company"),
        }
    }
}

#[tokio::test]
#[ignore]
async fn live_profile_fund_for_record() {
    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        return;
    }
    let mut client = yfinance_rs::YfClient::builder().build().unwrap();
    let _ = yfinance_rs::Profile::load(&mut client, "QQQ").await;
}

#[tokio::test]
#[ignore]
async fn live_profile_company_scrape_for_record() {
    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        return;
    }
    let mut client = yfinance_rs::YfClient::builder()
        .api_preference(yfinance_rs::ApiPreference::ScrapeOnly)
        .build()
        .unwrap();
    let _ = yfinance_rs::Profile::load(&mut client, "AAPL").await;
}

#[tokio::test]
#[ignore]
async fn live_profile_fund_scrape_for_record() {
    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        return;
    }
    let mut client = yfinance_rs::YfClient::builder()
        .api_preference(yfinance_rs::ApiPreference::ScrapeOnly)
        .build()
        .unwrap();
    let _ = yfinance_rs::Profile::load(&mut client, "QQQ").await;
}
