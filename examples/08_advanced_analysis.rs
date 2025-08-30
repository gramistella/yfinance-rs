use chrono::TimeZone;
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();

    println!("--- Fetching Advanced Analysis for AAPL ---");
    let ticker_aapl = Ticker::new(client.clone(), "AAPL");

    let earnings_trend = ticker_aapl.earnings_trend().await?;
    println!("Earnings Trend ({} periods):", earnings_trend.len());
    if let Some(trend) = earnings_trend.iter().find(|t| t.period == "0y") {
        println!(
            "  Current Year ({}): Earnings Est. Avg: {:.2}, Revenue Est. Avg: {}",
            trend.period,
            trend.earnings_estimate_avg.unwrap_or_default(),
            trend.revenue_estimate_avg.unwrap_or_default()
        );
    }
    println!();

    println!("--- Fetching Historical Shares for AAPL ---");
    let shares = ticker_aapl.shares().await?;
    println!("Annual Shares Outstanding ({} periods):", shares.len());
    if let Some(share_count) = shares.first() {
        println!(
            "  Latest Period ({}): {} shares",
            share_count.date, share_count.shares
        );
    }
    println!();

    println!("--- Fetching Capital Gains for VFINX (Vanguard 500 Index Fund) ---");
    let ticker_vfinx = Ticker::new(client, "VFINX");
    let capital_gains = ticker_vfinx.capital_gains(None).await?;
    println!(
        "Capital Gains Distributions ({} periods):",
        capital_gains.len()
    );
    if let Some((date, gain)) = capital_gains.last() {
        println!("  Most Recent Gain: ${:.2} on {}", gain, date);
    }

    println!("--- Analyst Price Target for AAPL ---");
    let price_target = ticker_aapl.analyst_price_target().await?;
    println!(
        "  Target: avg=${:.2}, high=${:.2}, low=${:.2} (from {} analysts)",
        price_target.mean.unwrap_or_default(),
        price_target.high.unwrap_or_default(),
        price_target.low.unwrap_or_default(),
        price_target.number_of_analysts.unwrap_or_default()
    );
    println!();

    println!("--- Recommendation Summary for AAPL ---");
    let rec_summary = ticker_aapl.recommendations_summary().await?;
    println!(
        "  Mean score: {:.2} ({})",
        rec_summary.mean.unwrap_or_default(),
        rec_summary.mean_key.unwrap_or_default()
    );
    println!();

    println!("--- Earnings Trend ({} periods):", earnings_trend.len());
    if let Some(trend) = earnings_trend.iter().find(|t| t.period == "0y") {
        println!(
            "  Current Year ({}): Earnings Est. Avg: {:.2}, Revenue Est. Avg: {}",
            trend.period,
            trend.earnings_estimate_avg.unwrap_or_default(),
            trend.revenue_estimate_avg.unwrap_or_default()
        );
    }
    println!();

    println!("--- Fetching Historical Shares for AAPL ---");
    let shares = ticker_aapl.shares().await?;
    println!("Annual Shares Outstanding ({} periods):", shares.len());
    if let Some(share_count) = shares.first() {
        println!(
            "  Latest Period ({}): {} shares",
            share_count.date, share_count.shares
        );
    }
    println!();

    println!("--- ISIN for AAPL ---");
    let isin = ticker_aapl.isin().await?;
    println!("  ISIN: {}", isin.unwrap_or("Not found".to_string()));
    println!();

    println!("--- Upcoming Calendar Events for AAPL ---");
    let calendar = ticker_aapl.calendar().await?;
    if let Some(date) = calendar.earnings_dates.first() {
        println!(
            "  Next earnings date (approx): {}",
            chrono::Utc.timestamp_opt(*date, 0).unwrap().date_naive()
        );
    }
    if let Some(date) = calendar.ex_dividend_date {
        println!(
            "  Ex-dividend date: {}",
            chrono::Utc.timestamp_opt(date, 0).unwrap().date_naive()
        );
    }
    println!();

    Ok(())
}
