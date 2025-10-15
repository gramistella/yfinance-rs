use yfinance_rs::core::conversions::money_to_f64;
use yfinance_rs::{FundamentalsBuilder, HoldersBuilder, SearchBuilder, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let symbol = "MSFT";

    // --- Part 1: Fetching Fundamentals ---
    println!("--- Fetching Fundamentals for {symbol} ---");
    let fundamentals = FundamentalsBuilder::new(&client, symbol);

    let annual_income_stmt = fundamentals.income_statement(false, None).await?;
    println!(
        "Latest Annual Income Statement ({} periods):",
        annual_income_stmt.len()
    );
    if let Some(stmt) = annual_income_stmt.first() {
        println!(
            "  Period End: {} | Total Revenue: {:.2}",
            stmt.period,
            stmt.total_revenue
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default()
        );
    }

    let quarterly_balance_sheet = fundamentals.balance_sheet(true, None).await?;
    println!(
        "Latest Quarterly Balance Sheet ({} periods):",
        quarterly_balance_sheet.len()
    );
    if let Some(stmt) = quarterly_balance_sheet.first() {
        println!(
            "  Period End: {} | Total Assets: {:.2}",
            stmt.period,
            stmt.total_assets
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default()
        );
    }

    let earnings = fundamentals.earnings(None).await?;
    println!("Latest Earnings Summary:");
    if let Some(e) = earnings.quarterly.first() {
        println!(
            "  Quarter {}: Revenue: {:.2} | Earnings: {:.2}",
            e.period,
            e.revenue.as_ref().map(money_to_f64).unwrap_or_default(),
            e.earnings.as_ref().map(money_to_f64).unwrap_or_default()
        );
    }
    println!("--------------------------------------\n");

    // --- Part 2: Fetching Holder Information ---
    println!("--- Fetching Holder Info for {symbol} ---");
    let holders_builder = HoldersBuilder::new(&client, symbol);

    let major_holders = holders_builder.major_holders().await?;
    println!("Major Holders Breakdown:");
    for holder in major_holders {
        println!("  {}: {}", holder.category, holder.value);
    }

    let inst_holders = holders_builder.institutional_holders().await?;
    println!("\nTop 5 Institutional Holders:");
    for holder in inst_holders.iter().take(5) {
        println!(
            "  - {}: {:?} shares ({:?}%)",
            holder.holder, holder.shares, holder.pct_held
        );
    }

    let net_activity = holders_builder.net_share_purchase_activity().await?;
    if let Some(activity) = net_activity {
        println!("\nNet Insider Purchase Activity ({}):", activity.period);
        println!("  Net shares bought/sold: {:?}", activity.net_shares);
    }
    println!("--------------------------------------\n");

    // --- Part 3: Searching for Tickers ---
    let query = "S&P 500";
    println!("--- Searching for '{query}' ---");

    let search_results = SearchBuilder::new(&client, query).fetch().await?;

    println!("Found {} results:", search_results.results.len());
    for quote in search_results.results {
        let name = quote.name.unwrap_or_default();
        let exchange = quote.exchange.map(|e| e.to_string()).unwrap_or_default();
        let kind = quote.kind.to_string();
        println!("  - {}: {} ({}) on {}", quote.symbol, name, kind, exchange);
    }
    println!("--------------------------------------");

    Ok(())
}
