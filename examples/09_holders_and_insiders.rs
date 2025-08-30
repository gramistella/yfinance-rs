use chrono::TimeZone;
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(client, "TSLA");

    println!("--- Fetching Holder Information for TSLA ---");

    // Mutual Fund Holders
    let mf_holders = ticker.mutual_fund_holders().await?;
    println!("\nTop 5 Mutual Fund Holders:");
    for holder in mf_holders.iter().take(5) {
        println!(
            "  - {}: {} shares ({:.2}%)",
            holder.holder,
            holder.shares,
            holder.pct_held * 100.0
        );
    }

    // Insider Transactions
    let insider_txns = ticker.insider_transactions().await?;
    println!("\nLatest 5 Insider Transactions:");
    for txn in insider_txns.iter().take(5) {
        println!(
            "  - {}: {} {} shares on {}",
            txn.insider,
            txn.transaction,
            txn.shares,
            chrono::Utc
                .timestamp_opt(txn.start_date, 0)
                .unwrap()
                .date_naive()
        );
    }

    // Insider Roster
    let insider_roster = ticker.insider_roster_holders().await?;
    println!("\nTop 5 Insider Roster:");
    for insider in insider_roster.iter().take(5) {
        println!(
            "  - {} ({}): {} shares",
            insider.name, insider.position, insider.shares_owned_directly
        );
    }

    println!("-----------------------------------------");

    Ok(())
}
