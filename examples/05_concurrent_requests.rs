use chrono::TimeZone;
use futures::future::try_join_all;
use yfinance_rs::{FundamentalsBuilder, SearchBuilder, Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let symbols = ["AAPL", "GOOGL", "TSLA"];

    println!("--- Fetching a comprehensive overview for multiple tickers ---");
    let fetch_info_tasks: Vec<_> = symbols
        .iter()
        .map(|&s| {
            let ticker = Ticker::new(client.clone(), s);
            async move {
                let info = ticker.info().await?;
                println!(
                    "Symbol: {}, Name: {}, Price: {:.2}, Sector: {:?}",
                    info.symbol,
                    info.short_name.unwrap_or_default(),
                    info.regular_market_price.unwrap_or(0.0),
                    info.sector
                );
                Ok::<_, yfinance_rs::YfError>(())
            }
        })
        .collect();
    let _ = try_join_all(fetch_info_tasks).await?;
    println!();

    println!("--- Fetching annual fundamentals for a single ticker (AAPL) ---");
    let aapl_fundamentals = FundamentalsBuilder::new(client.clone(), "AAPL");
    let annual_income_stmt = aapl_fundamentals.income_statement(false).await?;
    if let Some(stmt) = annual_income_stmt.first() {
        println!(
            "AAPL Latest Annual Revenue: {:.2} (from {})",
            stmt.total_revenue.unwrap_or_default(),
            chrono::Utc
                .timestamp_opt(stmt.period_end, 0)
                .unwrap()
                .date_naive()
        );
    }
    let annual_cashflow = aapl_fundamentals.cashflow(false).await?;
    if let Some(cf) = annual_cashflow.first() {
        println!(
            "AAPL Latest Annual Free Cash Flow: {:.2}",
            cf.free_cash_flow.unwrap_or_default()
        );
    }
    println!();

    println!("--- Fetching ESG and holder data for MSFT ---");
    let msft_ticker = Ticker::new(client.clone(), "MSFT");
    let esg_scores = msft_ticker.sustainability().await?;
    println!(
        "MSFT Total ESG Score: {:.2}",
        esg_scores.total_esg.unwrap_or_default()
    );
    let institutional_holders = msft_ticker.institutional_holders().await?;
    if let Some(holder) = institutional_holders.first() {
        println!(
            "MSFT Top institutional holder: {} with {} shares",
            holder.holder, holder.shares
        );
    }
    println!();

    println!("--- Searching for SPY and getting its ticker ---");
    let search_results = SearchBuilder::new(client.clone(), "SPY")?.fetch().await?;
    if let Some(sp500_quote) = search_results.quotes.iter().find(|q| q.symbol == "SPY") {
        println!(
            "Found: {} ({})",
            sp500_quote.longname.as_deref().unwrap_or("N/A"),
            sp500_quote.symbol
        );
    }
    println!();

    Ok(())
}
