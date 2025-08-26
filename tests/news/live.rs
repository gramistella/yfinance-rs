use yfinance_rs::{NewsTab, Ticker, YfClient};

#[tokio::test]
#[ignore]
async fn live_news_smoke_and_or_record() {
    if !crate::common::live_or_record_enabled() {
        return;
    }

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::new(client, "AAPL");

    // This call will record `tests/fixtures/news_AAPL.json` if YF_RECORD=1
    let news = ticker.news().await.unwrap();

    if !crate::common::is_recording() {
        // Basic sanity checks when running in live-only mode
        assert!(
            !news.is_empty(),
            "Expected to get at least one news article for AAPL"
        );
        let article = &news[0];
        assert!(!article.uuid.is_empty());
        assert!(!article.title.is_empty());
        assert!(article.provider_publish_time > 1_000_000_000); // Sanity check timestamp
    }
}

#[tokio::test]
#[ignore]
async fn live_news_press_releases_for_record() {
    if !crate::common::is_recording() {
        return;
    }

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::new(client, "AAPL");

    // This will record the fixture `news_pressReleases_AAPL.json`
    let _ = ticker
        .news_builder()
        .tab(NewsTab::PressReleases)
        .fetch()
        .await
        .unwrap();
}
