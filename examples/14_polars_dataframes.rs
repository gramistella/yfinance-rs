//! Example demonstrating Polars `DataFrame` integration with yfinance-rs.
//!
//! Run with: cargo run --example 14_polars_dataframes --features dataframe

#[cfg(feature = "dataframe")]
use polars::prelude::*;

#[cfg(feature = "dataframe")]
use paft::core::dataframe::{ToDataFrame, ToDataFrameVec};

#[cfg(feature = "dataframe")]
use yfinance_rs::{Ticker, YfClient};

#[cfg(feature = "dataframe")]
use yfinance_rs::core::{Interval, Range};

#[cfg(feature = "dataframe")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();

    println!("=== Polars DataFrame Integration with yfinance-rs ===\n");

    let ticker = Ticker::new(&client, "AAPL");
    section_history_df(&ticker).await?;
    section_quote_df(&ticker).await?;
    section_recommendations_df(&ticker).await?;
    section_income_df(&ticker).await?;
    section_esg(&ticker).await?;
    section_holders_df(&ticker).await?;
    section_analysis_df(&ticker).await?;

    println!("\n=== DataFrame Integration Complete ===");
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_history_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 1. Historical Price Data to DataFrame");
    let history = ticker
        .history(Some(Range::M6), Some(Interval::D1), false)
        .await?;

    if !history.is_empty() {
        let df = history.to_dataframe()?;
        println!("   DataFrame shape: {:?}", df.shape());
        println!("   Sample data:\n{}", df.head(Some(5)));
    } else {
        println!("   No history returned.");
    }
    println!();
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_quote_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 2. Current Quote to DataFrame");
    match ticker.quote().await {
        Ok(quote) => {
            let df = quote.to_dataframe()?;
            println!("   DataFrame shape: {:?}", df.shape());
            println!("   Quote data:\n{df}");
        }
        Err(e) => println!("   Error fetching quote: {e}"),
    }
    println!();
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_recommendations_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧾 3. Analyst Recommendations to DataFrame");
    match ticker.recommendations().await {
        Ok(recommendations) => {
            if recommendations.is_empty() {
                println!("   No recommendation data available");
            } else {
                let df = recommendations.to_dataframe()?;
                println!("   DataFrame shape: {:?}", df.shape());
                println!("   Recommendation data:\n{}", df.head(Some(5)));
            }
        }
        Err(e) => println!("   Error fetching recommendations: {e}"),
    }
    println!();
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_income_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("💰 4. Financial Statements to DataFrame");
    match ticker.income_stmt(None).await {
        Ok(financials) => {
            if financials.is_empty() {
                println!("   No financial data available");
            } else {
                let df = financials.to_dataframe()?;
                println!("   DataFrame shape: {:?}", df.shape());
                println!("   Income statement data:\n{}", df.head(Some(3)));
            }
        }
        Err(e) => println!("   Error fetching financials: {e}"),
    }
    println!();
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_esg(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌱 5. ESG Scores");
    match ticker.sustainability().await {
        Ok(summary) => {
            if let Some(scores) = summary.scores {
                println!("   Environmental: {:?}", scores.environmental);
                println!("   Social: {:?}", scores.social);
                println!("   Governance: {:?}", scores.governance);
            } else {
                println!("   No ESG component scores available");
            }
        }
        Err(e) => println!("   ESG data not available for this ticker: {e}"),
    }
    println!();
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_holders_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏛️ 6. Institutional Holders to DataFrame");
    match ticker.institutional_holders().await {
        Ok(holders) => {
            if holders.is_empty() {
                println!("   No institutional holders data available");
            } else {
                let df = holders.to_dataframe()?;
                println!("   DataFrame shape: {:?}", df.shape());
                println!("   Top institutional holders:\n{}", df.head(Some(5)));
            }
        }
        Err(e) => println!("   Institutional holders data not available: {e}"),
    }
    println!();
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_analysis_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 7. Simple Analysis with Polars");
    let history = ticker
        .history(Some(Range::M6), Some(Interval::D1), false)
        .await?;
    if history.is_empty() {
        println!("   No history for analysis.");
        return Ok(());
    }
    let df = history.to_dataframe()?;

    // Lazily compute a few stats
    let lf = df.lazy();
    let stats = lf
        .clone()
        .select([
            col("close.amount").mean().alias("avg_close"),
            col("close.amount").min().alias("min_close"),
            col("close.amount").max().alias("max_close"),
            col("volume").sum().alias("total_volume"),
        ])
        .collect()?;
    println!("   6M Close/Volume Stats:\n{stats}");

    let with_ma = lf
        .sort(["ts"], SortMultipleOptions::default())
        .with_column(
            col("close.amount")
                .rolling_mean(RollingOptionsFixedWindow {
                    window_size: 5,
                    min_periods: 1,
                    ..Default::default()
                })
                .alias("ma_5d"),
        )
        .select([col("ts"), col("close.amount"), col("ma_5d"), col("volume")])
        .limit(10)
        .collect()?;
    println!("   First 10 rows with 5-day moving average:\n{with_ma}");
    Ok(())
}

#[cfg(not(feature = "dataframe"))]
fn main() {
    println!("This example requires the 'dataframe' feature to be enabled.");
    println!("Run with: cargo run --example 14_polars_dataframes --features dataframe");
}
