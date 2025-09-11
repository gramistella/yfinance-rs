#[tokio::test]
#[ignore]
async fn live_profile_company() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let prof = yfinance_rs::profile::load_profile(&client, "AAPL")
        .await
        .unwrap();

    if !crate::common::is_recording() {
        match prof {
            yfinance_rs::Profile::Company(c) => assert_eq!(c.name, "Apple Inc."),
            _ => panic!("expected Company"),
        }
    }
}

#[tokio::test]
#[ignore]
async fn live_profile_fund_for_record() {
    if !crate::common::is_recording() {
        return;
    }
    let client = yfinance_rs::YfClient::builder().build().unwrap();
    let _ = yfinance_rs::profile::load_profile(&client, "QQQ").await;
}

#[tokio::test]
#[ignore]
async fn live_profile_company_scrape_for_record() {
    if !crate::common::is_recording() {
        return;
    }
    let client = yfinance_rs::YfClient::builder()
        ._api_preference(yfinance_rs::ApiPreference::ScrapeOnly)
        .build()
        .unwrap();
    let _ = yfinance_rs::profile::load_profile(&client, "AAPL").await;
}

#[tokio::test]
#[ignore]
async fn live_profile_fund_scrape_for_record() {
    if !crate::common::is_recording() {
        return;
    }
    let client = yfinance_rs::YfClient::builder()
        ._api_preference(yfinance_rs::ApiPreference::ScrapeOnly)
        .build()
        .unwrap();
    let _ = yfinance_rs::profile::load_profile(&client, "QQQ").await;
}
