use std::time::Duration;
use yfinance_rs::{
    Ticker, YfClientBuilder, YfError,
    core::client::{Backoff, RetryConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. --- Advanced Client Configuration ---
    println!("--- Building a client with custom configuration ---");
    let custom_retry = RetryConfig {
        enabled: true,
        max_retries: 2,
        backoff: Backoff::Fixed(Duration::from_millis(500)),
        ..Default::default()
    };
    let client = YfClientBuilder::default()
        .retry_config(custom_retry)
        .cache_ttl(Duration::from_secs(60)) // Cache responses for 60 seconds
        .build()?;
    println!("Client built with custom retry policy.");
    println!();

    // 2. --- Using the custom client ---
    let aapl = Ticker::new(&client, "AAPL");
    let quote1 = aapl.quote().await?;
    println!(
        "First fetch for {}: ${:.2} (from network)",
        quote1.symbol,
        quote1.regular_market_price.unwrap_or_default()
    );
    let quote2 = aapl.quote().await?;
    println!(
        "Second fetch for {}: ${:.2} (should be from cache)",
        quote2.symbol,
        quote2.regular_market_price.unwrap_or_default()
    );
    println!();

    // 3. --- Cache Management ---
    println!("--- Managing the client cache ---");
    client.clear_cache().await;
    println!("Client cache cleared.");
    let quote3 = aapl.quote().await?;
    println!(
        "Third fetch for {}: ${:.2} (from network again)",
        quote3.symbol,
        quote3.regular_market_price.unwrap_or_default()
    );
    println!();

    // 4. --- Demonstrating a missing data point (dividend date) ---
    println!("--- Fetching Calendar Events for AAPL (including dividend date) ---");
    let calendar = aapl.calendar().await?;
    if let Some(date) = calendar.dividend_date {
        use chrono::{TimeZone, Utc};
        println!(
            "  Dividend date: {}",
            Utc.timestamp_opt(date, 0).unwrap().date_naive()
        );
    } else {
        println!("  No upcoming dividend date found.");
    }
    println!();

    // 5. --- Error Handling Example ---
    println!("--- Handling a non-existent ticker ---");
    let bad_ticker = Ticker::new(&client, "THIS-TICKER-DOES-NOT-EXIST-XYZ");
    match bad_ticker.info().await {
        Ok(_) => println!("Unexpected success fetching bad ticker."),
        Err(YfError::MissingData(msg)) => {
            println!("Correctly failed with a missing data error: {}", msg);
        }
        Err(e) => {
            println!("Failed with an unexpected error type: {}", e);
        }
    }

    Ok(())
}
