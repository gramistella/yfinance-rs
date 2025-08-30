use chrono::{TimeZone, Utc};
use yfinance_rs::{Interval, Range, Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(client, "AAPL");

    println!("--- Ticker Quote (Convenience) ---");
    let quote = ticker.quote().await?;
    println!(
        "  {}: ${:.2} (prev_close: ${:.2})",
        quote.symbol,
        quote.regular_market_price.unwrap_or_default(),
        quote.regular_market_previous_close.unwrap_or_default()
    );
    println!();

    println!("--- Ticker News (Convenience, default count) ---");
    let news = ticker.news().await?;
    println!("  Found {} articles with default settings.", news.len());
    if let Some(article) = news.first() {
        println!("  First article: {}", article.title);
    }
    println!();

    println!("--- Ticker History (Convenience, last 5 days) ---");
    let history = ticker
        .history(Some(Range::D5), Some(Interval::D1), false)
        .await?;
    if let Some(candle) = history.last() {
        println!(
            "  Last close on {}: ${:.2}",
            Utc.timestamp_opt(candle.ts, 0).unwrap().date_naive(),
            candle.close
        );
    }
    println!();

    println!("--- Ticker Actions (Convenience, YTD) ---");
    let actions = ticker.actions(Some(Range::Ytd)).await?;
    println!("  Found {} actions (dividends/splits) YTD.", actions.len());
    if let Some(action) = actions.last() {
        println!("  Most recent action: {:?}", action);
    }
    println!();

    println!("--- Annual Financials (Convenience) ---");
    let annual_income = ticker.income_stmt().await?;
    if let Some(stmt) = annual_income.first() {
        println!(
            "  Latest annual revenue: {:.2}",
            stmt.total_revenue.unwrap_or_default()
        );
    }

    let annual_balance = ticker.balance_sheet().await?;
    if let Some(stmt) = annual_balance.first() {
        println!(
            "  Latest annual assets: {:.2}",
            stmt.total_assets.unwrap_or_default()
        );
    }

    let annual_cashflow = ticker.cashflow().await?;
    if let Some(stmt) = annual_cashflow.first() {
        println!(
            "  Latest annual free cash flow: {:.2}",
            stmt.free_cash_flow.unwrap_or_default()
        );
    }

    Ok(())
}
