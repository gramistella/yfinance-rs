use std::time;

use chrono::{Duration, Utc};
use yfinance_rs::core::Interval;
use yfinance_rs::{
    DownloadBuilder, QuotesBuilder, SearchBuilder, Ticker, YfClient,
    core::client::{Backoff, CacheMode, RetryConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();

    println!("--- QuotesBuilder Usage ---");
    let quotes = QuotesBuilder::new(client.clone())
        .symbols(vec!["F", "GM", "TSLA"])
        .fetch()
        .await?;
    println!("  Fetched {} quotes via QuotesBuilder.", quotes.len());
    println!();

    println!("--- Per-Request Configuration: No Cache ---");
    let aapl = Ticker::new(&client, "AAPL").cache_mode(CacheMode::Bypass);
    let quote_no_cache = aapl.quote().await?;
    println!(
        "  Fetched {} quote, bypassing the client's cache.",
        quote_no_cache.symbol
    );
    println!();

    println!("--- SearchBuilder Customization ---");
    let search_results = SearchBuilder::new(&client, "Microsoft")
        .quotes_count(2)
        .region("US")
        .lang("en-US")
        .fetch()
        .await?;
    println!(
        "  Found {} results for 'Microsoft' in US region.",
        search_results.results.len()
    );
    for quote in search_results.results {
        println!(
            "    - {} ({})",
            quote.symbol,
            quote.name.unwrap_or_default()
        );
    }
    println!();

    println!("--- DownloadBuilder with pre/post market and keepna ---");
    // Get recent data including pre/post market, which might have gaps (keepna=true)
    let today = Utc::now();
    let yesterday = today - Duration::days(1);
    let download = DownloadBuilder::new(&client)
        .symbols(vec!["TSLA"])
        .between(yesterday, today)
        .interval(Interval::I15m)
        .prepost(true)
        .keepna(true)
        .run()
        .await?;
    if let Some(candles) = download.series.get("TSLA") {
        println!(
            "  Fetched {} 15m candles for TSLA in the last 24h (pre/post included).",
            candles.len()
        );
    }
    println!();

    println!("--- Overriding Retry Policy for a Single Ticker ---");
    let custom_retry = RetryConfig {
        enabled: true,
        max_retries: 1,
        backoff: Backoff::Fixed(time::Duration::from_millis(100)),
        ..Default::default()
    };
    let goog = Ticker::new(&client, "GOOG").retry_policy(Some(custom_retry));
    // This call will now use the custom retry policy instead of the client's default
    let goog_info = goog.fast_info().await?;
    println!(
        "  Fetched fast info for {} with a custom retry policy.",
        goog_info.symbol
    );

    Ok(())
}
