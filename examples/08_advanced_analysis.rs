use yfinance_rs::core::conversions::money_to_f64;
use yfinance_rs::{Ticker, YfClient, YfError};

#[tokio::main]
async fn main() -> Result<(), YfError> {
    let client = YfClient::default();

    let symbol = "AAPL";
    let ticker_aapl = Ticker::new(&client, symbol);
    section_earnings_and_shares(symbol, &ticker_aapl).await?;
    section_capital_gains().await?;
    section_price_target(symbol, &ticker_aapl).await?;
    section_recommendations(symbol, &ticker_aapl).await?;
    section_isin_calendar(symbol, &ticker_aapl).await?;
    Ok(())
}

async fn section_earnings_and_shares(symbol: &str, ticker: &Ticker) -> Result<(), YfError> {
    println!("--- Fetching Advanced Analysis for {symbol} ---");
    let earnings_trend = ticker.earnings_trend(None).await?;
    println!("Earnings Trend ({} periods):", earnings_trend.len());
    if let Some(trend) = earnings_trend.iter().find(|t| t.period.to_string() == "0y") {
        println!(
            "  Current Year ({}): Earnings Est. Avg: {:.2}, Revenue Est. Avg: {}",
            trend.period,
            trend
                .earnings_estimate
                .avg
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default(),
            trend
                .revenue_estimate
                .avg
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default()
        );
    }
    println!();

    println!("--- Fetching Historical Shares for {symbol} ---");
    let shares = ticker.shares().await?;
    println!("Annual Shares Outstanding ({} periods):", shares.len());
    if let Some(share_count) = shares.first() {
        println!(
            "  Latest Period ({}): {} shares",
            share_count.date, share_count.shares
        );
    }
    println!();
    Ok(())
}

async fn section_capital_gains() -> Result<(), YfError> {
    println!("--- Fetching Capital Gains for VFINX (Vanguard 500 Index Fund) ---");
    let client = YfClient::default();
    let ticker_vfinx = Ticker::new(&client, "VFINX");
    let capital_gains = ticker_vfinx.capital_gains(None).await?;
    println!(
        "Capital Gains Distributions ({} periods):",
        capital_gains.len()
    );
    if let Some((date, gain)) = capital_gains.last() {
        println!("  Most Recent Gain: ${gain:.2} on {date}");
    }
    Ok(())
}

async fn section_price_target(symbol: &str, ticker: &Ticker) -> Result<(), YfError> {
    println!("--- Analyst Price Target for {symbol} ---");
    let price_target = ticker.analyst_price_target(None).await?;
    println!(
        "  Target: avg=${:.2}, high=${:.2}, low=${:.2} (from {} analysts)",
        price_target
            .mean
            .as_ref()
            .map(money_to_f64)
            .unwrap_or_default(),
        price_target
            .high
            .as_ref()
            .map(money_to_f64)
            .unwrap_or_default(),
        price_target
            .low
            .as_ref()
            .map(money_to_f64)
            .unwrap_or_default(),
        price_target.number_of_analysts.unwrap_or_default()
    );
    println!();
    Ok(())
}

async fn section_recommendations(symbol: &str, ticker: &Ticker) -> Result<(), YfError> {
    println!("--- Recommendation Summary for {symbol} ---");
    let rec_summary = ticker.recommendations_summary().await?;
    println!(
        "  Mean score: {:.2} ({})",
        rec_summary.mean.unwrap_or_default(),
        rec_summary.mean_rating_text.as_deref().unwrap_or("N/A")
    );
    println!();
    Ok(())
}

async fn section_isin_calendar(symbol: &str, ticker: &Ticker) -> Result<(), YfError> {
    println!("--- ISIN for {symbol} ---");
    let isin = ticker.isin().await?;
    println!(
        "  ISIN: {}",
        isin.unwrap_or_else(|| "Not found".to_string())
    );
    println!();

    println!("--- Upcoming Calendar Events for {symbol} ---");
    let calendar = ticker.calendar().await?;
    if let Some(date) = calendar.earnings_dates.first() {
        println!("  Next earnings date (approx): {}", date.date_naive());
    }
    if let Some(date) = calendar.ex_dividend_date {
        println!("  Ex-dividend date: {}", date.date_naive());
    }
    println!();
    Ok(())
}
