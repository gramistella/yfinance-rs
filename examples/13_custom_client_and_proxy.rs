use reqwest::Client;
use std::time::Duration;
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Custom Client and Proxy Configuration Examples ===\n");

    // Example 1: Using a custom reqwest client for full control
    println!("1. Custom Reqwest Client Example:");
    let custom_client = Client::builder()
        // Set user agent to avoid 429 errors
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        // You must enable cookie storage to avoid 403 Invalid Cookie errors
        .cookie_store(true)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(90))
        .build()?;

    let client_with_custom = YfClient::builder().custom_client(custom_client).build()?;

    let ticker = Ticker::new(&client_with_custom, "AAPL");
    match ticker.quote().await {
        Ok(quote) => println!(
            "   Fetched quote for {} using custom client",
            quote.instrument
        ),
        Err(e) => println!("   Rate limited or error fetching quote: {e}"),
    }
    println!();

    // Example 2: Using HTTP proxy through builder
    println!("2. HTTP Proxy Configuration Example:");
    // Note: This example uses a dummy proxy URL - replace with actual proxy if needed
    // let client_with_proxy = YfClient::builder()
    //     .proxy("http://proxy.example.com:8080")
    //     .timeout(Duration::from_secs(30))
    //     .build()?;

    // For demonstration, we'll show the builder pattern without actually using a proxy
    let client_with_timeout = YfClient::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()?;

    let ticker = Ticker::new(&client_with_timeout, "MSFT");
    match ticker.quote().await {
        Ok(quote) => println!(
            "   Fetched quote for {} with custom timeout",
            quote.instrument
        ),
        Err(e) => println!("   Rate limited or error fetching quote: {e}"),
    }
    println!();

    // Example 3: Using HTTPS proxy with error handling
    println!("3. HTTPS Proxy with Error Handling Example:");
    // Note: This example shows the pattern but uses a dummy URL
    // let client_with_https_proxy = YfClient::builder()
    //     .try_https_proxy("https://proxy.example.com:8443")?
    //     .timeout(Duration::from_secs(30))
    //     .build()?;

    // For demonstration, we'll show the error handling pattern
    let client_with_retry = YfClient::builder()
        .timeout(Duration::from_secs(30))
        .retry_enabled(true)
        .build()?;

    let ticker = Ticker::new(&client_with_retry, "GOOGL");
    match ticker.quote().await {
        Ok(quote) => println!(
            "   Fetched quote for {} with retry enabled",
            quote.instrument
        ),
        Err(e) => println!("   Rate limited or error fetching quote: {e}"),
    }
    println!();

    // Example 4: Advanced custom client configuration
    println!("4. Advanced Custom Client Configuration:");
    let advanced_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(120))
        .pool_max_idle_per_host(10)
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .build()?;

    let client_with_advanced = YfClient::builder()
        .custom_client(advanced_client)
        .cache_ttl(Duration::from_secs(300)) // 5 minutes cache
        .build()?;

    let ticker = Ticker::new(&client_with_advanced, "TSLA");
    match ticker.quote().await {
        Ok(quote) => println!(
            "   Fetched quote for {} with advanced client config",
            quote.instrument
        ),
        Err(e) => println!("   Rate limited or error fetching quote: {e}"),
    }
    println!();

    // Example 5: Error handling for invalid proxy URLs
    println!("5. Error Handling for Invalid Proxy URLs:");
    match YfClient::builder().try_proxy("invalid-url") {
        Ok(_) => println!("   Unexpected: Invalid proxy URL was accepted"),
        Err(e) => println!("   Expected error for invalid proxy URL: {e}"),
    }

    match YfClient::builder().try_https_proxy("not-a-url") {
        Ok(_) => println!("   Unexpected: Invalid HTTPS proxy URL was accepted"),
        Err(e) => println!("   Expected error for invalid HTTPS proxy URL: {e}"),
    }
    println!();

    // Example 6: Builder pattern validation
    println!("6. Builder Pattern Validation:");
    let client = YfClient::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .retry_enabled(true)
        .cache_ttl(Duration::from_secs(60))
        .build()?;

    println!("   Successfully built client with custom configuration");
    println!("   - Retry config: {:?}", client.retry_config());
    println!();

    // Example 7: Working HTTPS proxy example (commented out for safety)
    // Uncomment and replace with your actual proxy URL:
    // let client_with_https = YfClient::builder()
    //     .https_proxy("https://your-proxy.com:8443")
    //     .timeout(Duration::from_secs(30))
    //     .build()?;

    println!("=== All examples completed successfully! ===");
    println!();
    println!("Key points:");
    println!("- Use .custom_client() for full reqwest control");
    println!("- Use .proxy() for HTTP proxy setup");
    println!("- Use .https_proxy() for HTTPS proxy setup");
    println!("- Use .try_proxy() or .try_https_proxy() for error handling");
    println!("- Custom client takes precedence over other HTTP settings");
    println!("- Rate limiting (429) is common with live API calls");

    Ok(())
}
