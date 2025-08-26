use chrono::{Duration, Utc};
use yfinance_rs::{DownloadBuilder, Interval, Range, Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();

    // --- Part 1: Fetching Historical Dividends and Splits ---
    let aapl_ticker = Ticker::new(client.clone(), "AAPL");

    println!("--- Fetching Historical Actions for AAPL (last 5 years) ---");
    let dividends = aapl_ticker.dividends(Some(Range::Y5)).await?;
    println!("Found {} dividends in the last 5 years.", dividends.len());
    if let Some((ts, amount)) = dividends.last() {
        println!("  Latest dividend: ${:.2} on {}", amount, ts);
    }

    let splits = aapl_ticker.splits(Some(Range::Y5)).await?;
    println!("\nFound {} splits in the last 5 years.", splits.len());
    for (ts, num, den) in splits {
        println!("  - Split of {}:{} on {}", num, den, ts);
    }
    println!("--------------------------------------\n");

    // --- Part 2: Advanced Multi-Symbol Download with Customization ---
    let symbols = vec!["AAPL", "GOOGL", "MSFT", "AMZN"];
    println!("--- Downloading Custom Historical Data for Multiple Symbols ---");
    println!("Fetching 1-week, auto-adjusted data for the last 30 days...");

    let thirty_days_ago = Utc::now() - Duration::days(30);
    let now = Utc::now();

    let results = DownloadBuilder::new(&client)
        .symbols(symbols)
        .between(thirty_days_ago, now)
        .interval(Interval::W1) // Weekly interval
        .auto_adjust(true) // Adjust prices for splits/dividends
        .run()
        .await?;

    for (symbol, candles) in &results.series {
        println!("- {} ({} candles)", symbol, candles.len());
        if let Some(first_candle) = candles.first() {
            println!("  First Open: ${:.2}", first_candle.open);
        }
        if let Some(last_candle) = candles.last() {
            println!("  Last Close: ${:.2}", last_candle.close);
        }
    }
    println!("--------------------------------------");

    Ok(())
}
