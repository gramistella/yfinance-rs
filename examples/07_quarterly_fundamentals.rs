use chrono::TimeZone;
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "MSFT");

    println!("--- Fetching Quarterly Financial Statements for MSFT ---");
    println!("Fetching latest quarterly income statement...");
    let income_stmt = ticker.quarterly_income_stmt().await?;
    if let Some(latest) = income_stmt.first() {
        println!(
            "Latest quarterly revenue: {:.2} (from {})",
            latest.total_revenue.unwrap_or_default(),
            latest.period_end
        );
    } else {
        println!("No quarterly income statement found.");
    }

    println!("\nFetching latest quarterly balance sheet...");
    let balance_sheet = ticker.quarterly_balance_sheet().await?;
    if let Some(latest) = balance_sheet.first() {
        println!(
            "Latest quarterly total assets: {:.2} (from {})",
            latest.total_assets.unwrap_or_default(),
            latest.period_end
        );
    } else {
        println!("No quarterly balance sheet found.");
    }

    println!("\nFetching latest quarterly cash flow statement...");
    let cashflow_stmt = ticker.quarterly_cashflow().await?;
    if let Some(latest) = cashflow_stmt.first() {
        println!(
            "Latest quarterly operating cash flow: {:.2} (from {})",
            latest.operating_cashflow.unwrap_or_default(),
            latest.period_end
        );
    } else {
        println!("No quarterly cash flow statement found.");
    }

    println!("\nFetching latest quarterly shares outstanding...");
    let shares = ticker.quarterly_shares().await?;
    if let Some(latest) = shares.first() {
        println!(
            "Latest quarterly shares outstanding: {} (from {})",
            latest.shares,
            chrono::Utc
                .timestamp_opt(latest.date, 0)
                .unwrap()
                .date_naive()
        );
    } else {
        println!("No quarterly shares outstanding found.");
    }

    Ok(())
}
