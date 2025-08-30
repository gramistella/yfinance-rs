use chrono::{Duration, Utc};
use yfinance_rs::{
    DownloadBuilder, Interval, NewsTab, StreamBuilder, StreamMethod, Ticker,
    core::client::YfClientBuilder,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a customized client with a 5-second timeout and exponential backoff retries.
    let client = YfClientBuilder::default()
        .timeout(Duration::seconds(5).to_std()?)
        .build()?;

    // 2. Fetch a comprehensive summary for a ticker.
    let msft = Ticker::new(client.clone(), "MSFT");
    let info = msft.info().await?;
    println!("--- Ticker Info for {} ---", info.symbol);
    println!("Name: {}", info.short_name.unwrap_or_default());
    println!("Industry: {}", info.industry.unwrap_or_default());
    println!("Website: {}", info.website.unwrap_or_default());
    println!(
        "Mean Analyst Target: ${:.2}",
        info.target_mean_price.unwrap_or_default()
    );
    println!("ESG Score: {:.2}", info.total_esg_score.unwrap_or_default());
    println!();

    println!("--- Fast Info for NVDA ---");
    let nvda = Ticker::new(client.clone(), "NVDA");
    let fast_info = nvda.fast_info().await?;
    println!(
        "{} is trading at ${:.2} in {}",
        fast_info.symbol,
        fast_info.last_price,
        fast_info.exchange.unwrap_or_default()
    );
    println!();

    println!("--- Batch Quotes for Multiple Symbols ---");
    let quotes = yfinance_rs::quotes(&client, vec!["AMD", "INTC", "QCOM"]).await?;
    for quote in quotes {
        println!(
            "  {}: ${:.2}",
            quote.symbol,
            quote.regular_market_price.unwrap_or_default()
        );
    }
    println!();

    // 3. Download historical data for multiple tickers at once.
    let symbols = vec!["AAPL", "GOOG", "TSLA"];
    let today = Utc::now();
    let three_months_ago = today - Duration::days(90);
    println!("--- Historical Data for Multiple Symbols ---");
    let results = DownloadBuilder::new(&client)
        .symbols(symbols)
        .between(three_months_ago, today)
        .interval(Interval::D1)
        .run()
        .await?;

    for (symbol, candles) in &results.series {
        println!("{} has {} data points.", symbol, candles.len());
        if let Some(last_candle) = candles.last() {
            println!("  Last close price: ${:.2}", last_candle.close);
        }
    }
    println!();

    // 4. Fetch a specific options chain.
    let aapl = Ticker::new(client.clone(), "AAPL");
    let expirations = aapl.options().await?;
    if let Some(first_expiry) = expirations.first() {
        println!("--- Options Chain for AAPL ({first_expiry}) ---");
        let chain = aapl.option_chain(Some(*first_expiry)).await?;
        println!(
            "  Found {} calls and {} puts.",
            chain.calls.len(),
            chain.puts.len()
        );
        if let Some(first_call) = chain.calls.first() {
            println!(
                "  First call option: {} @ ${:.2}",
                first_call.contract_symbol, first_call.strike
            );
        }
    }
    println!();

    // 5. Stream real-time quotes using a WebSocket (with a fallback to polling if it fails).
    println!("--- Streaming Real-time Quotes for MSFT and GOOG ---");
    println!("(Streaming for 10 seconds or until stopped...)");
    let (handle, mut receiver) = StreamBuilder::new(&client)
        .symbols(vec!["MSFT", "GOOG"])
        .method(StreamMethod::WebsocketWithFallback)
        .start()?;

    let stream_task = tokio::spawn(async move {
        let mut count = 0;
        while let Some(update) = receiver.recv().await {
            println!(
                "[{}] {} @ {:.2} (ts={})",
                update.ts,
                update.symbol,
                update.last_price.unwrap_or_default(),
                update.ts
            );
            count += 1;
            if count >= 10 {
                break;
            }
        }
        println!("Finished streaming after {count} updates.");
    });

    // Stop the stream after 10 seconds, regardless of how many updates were received.
    tokio::select! {
        () = tokio::time::sleep(Duration::seconds(10).to_std()?) => {
            println!("Stopping stream due to timeout.");
            handle.stop().await;
        }
        _ = stream_task => {
            println!("Stream task completed on its own.");
        }
    };

    // 6. Fetching news articles for a ticker
    let tesla_news = Ticker::new(client, "TSLA");
    let articles = tesla_news
        .news_builder()
        .tab(NewsTab::PressReleases)
        .count(5)
        .fetch()
        .await?;
    println!("\n--- Latest 5 Press Releases for TSLA ---");
    for article in articles {
        println!(
            "- {} by {}",
            article.title,
            article.publisher.unwrap_or("Unknown".to_string())
        );
    }

    Ok(())
}
