use futures::future::try_join_all;
use yfinance_rs::core::conversions::money_to_f64;
use yfinance_rs::{FundamentalsBuilder, SearchBuilder, Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let symbols = ["AAPL", "GOOGL", "TSLA"];

    println!("--- Fetching a comprehensive overview for multiple tickers ---");
    let fetch_info_tasks: Vec<_> = symbols
        .iter()
        .map(|&s| {
            let ticker = Ticker::new(&client, s);
            async move {
                let info = ticker.info().await?;
                let vol = info
                    .volume
                    .map(|v| format!(" (vol: {v})"))
                    .unwrap_or_default();
                println!(
                    "Symbol: {}, Name: {}, Price: {:.2}{}",
                    info.symbol,
                    info.name.unwrap_or_default(),
                    info.last.as_ref().map_or(0.0, money_to_f64),
                    vol
                );
                Ok::<_, yfinance_rs::YfError>(())
            }
        })
        .collect();
    let _ = try_join_all(fetch_info_tasks).await?;
    println!();

    println!("--- Fetching annual fundamentals for a single ticker (AAPL) ---");
    let aapl_fundamentals = FundamentalsBuilder::new(&client, "AAPL");
    let annual_income_stmt = aapl_fundamentals.income_statement(false, None).await?;
    if let Some(stmt) = annual_income_stmt.first() {
        println!(
            "AAPL Latest Annual Revenue: {:.2} (from {})",
            stmt.total_revenue
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default(),
            stmt.period
        );
    }
    let annual_cashflow = aapl_fundamentals.cashflow(false, None).await?;
    if let Some(cf) = annual_cashflow.first() {
        println!(
            "AAPL Latest Annual Free Cash Flow: {:.2}",
            cf.free_cash_flow
                .as_ref()
                .map(money_to_f64)
                .unwrap_or_default()
        );
    }
    println!();

    println!("--- Fetching ESG and holder data for MSFT ---");
    let msft_ticker = Ticker::new(&client, "MSFT");
    let esg_summary = msft_ticker.sustainability().await?;
    let parts = esg_summary
        .scores
        .map_or([None, None, None], |s| {
            [s.environmental, s.social, s.governance]
        })
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let total_esg = if parts.is_empty() {
        0.0
    } else {
        let denom: f64 = u32::try_from(parts.len()).map(f64::from).unwrap_or(1.0);
        parts.iter().sum::<f64>() / denom
    };
    println!("MSFT Total ESG Score: {total_esg:.2}");
    let institutional_holders = msft_ticker.institutional_holders().await?;
    if let Some(holder) = institutional_holders.first() {
        println!(
            "MSFT Top institutional holder: {} with {:?} shares",
            holder.holder, holder.shares
        );
    }
    println!();

    println!("--- Searching for SPY and getting its ticker ---");
    let search_results = SearchBuilder::new(&client, "SPY").fetch().await?;
    if let Some(sp500_quote) = search_results
        .results
        .iter()
        .find(|q| q.symbol.as_str() == "SPY")
    {
        println!(
            "Found: {} ({})",
            sp500_quote.name.as_deref().unwrap_or("N/A"),
            sp500_quote.symbol
        );
    }
    println!();

    Ok(())
}
