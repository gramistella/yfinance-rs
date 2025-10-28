use yfinance_rs::core::conversions::money_to_f64;
use yfinance_rs::core::{Interval, Range};
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    println!("--- Ticker Quote (Convenience) ---");
    let quote = ticker.quote().await?;
    let vol = quote
        .day_volume
        .map(|v| format!(" (vol: {v})"))
        .unwrap_or_default();
    println!(
        "  {}: ${:.2} (prev_close: ${:.2}){}",
        quote.symbol,
        quote.price.as_ref().map(money_to_f64).unwrap_or_default(),
        quote
            .previous_close
            .as_ref()
            .map(money_to_f64)
            .unwrap_or_default(),
        vol
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
            candle.ts.date_naive(),
            money_to_f64(&candle.close)
        );
    }
    println!();

    println!("--- Ticker Actions (Convenience, YTD) ---");
    let actions = ticker.actions(Some(Range::Ytd)).await?;
    println!("  Found {} actions (dividends/splits) YTD.", actions.len());
    if let Some(action) = actions.last() {
        println!("  Most recent action: {action:?}");
    }
    println!();

    println!("--- Annual Financials (Convenience) ---");
    let annual_income = ticker.income_stmt(None).await?;
    if let Some(stmt) = annual_income.first() {
        println!(
            "  Latest annual revenue: {:.2}",
            stmt.total_revenue
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default()
        );
    }

    let annual_balance = ticker.balance_sheet(None).await?;
    if let Some(stmt) = annual_balance.first() {
        println!(
            "  Latest annual assets: {:.2}",
            stmt.total_assets
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default()
        );
    }

    let annual_cashflow = ticker.cashflow(None).await?;
    if let Some(stmt) = annual_cashflow.first() {
        println!(
            "  Latest annual free cash flow: {:.2}",
            stmt.free_cash_flow
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default()
        );
    }

    Ok(())
}
