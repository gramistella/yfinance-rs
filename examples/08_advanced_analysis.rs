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

    Ok(())
}
