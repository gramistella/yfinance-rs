use chrono::{Duration, Utc};
use yfinance_rs::{DownloadBuilder, Interval, Range, Ticker, YfClient};
use yfinance_rs::core::conversions::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();

    // --- Part 1: Fetching Historical Dividends and Splits ---
    let aapl_ticker = Ticker::new(&client, "AAPL");

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
        .interval(Interval::W1)
        .auto_adjust(true) // default, but explicit here
        .back_adjust(true) // show back-adjustment
        .repair(true) // show outlier repair
        .rounding(true) // show rounding
        .run()
        .await?;

    for (symbol, candles) in &results.series {
        println!("- {} ({} candles)", symbol, candles.len());
        if let Some(first_candle) = candles.first() {
            println!("  First Open: ${:.2}", money_to_f64(&first_candle.open));
        }
        if let Some(last_candle) = candles.last() {
            println!("  Last Close: ${:.2}", money_to_f64(&last_candle.close));
        }
    }
    println!("--------------------------------------");

    let meta = aapl_ticker.get_history_metadata(Some(Range::Y1)).await?;
    println!("\n--- History Metadata for AAPL ---");
    if let Some(m) = meta {
        println!("  Timezone: {}", m.timezone.unwrap_or_default());
        println!("  GMT Offset: {}", m.utc_offset_seconds.unwrap_or_default());
    }
    println!("--------------------------------------");

    Ok(())
}
