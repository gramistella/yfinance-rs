//! Example demonstrating Polars `DataFrame` integration with yfinance-rs.
//!
//! This example shows how to convert various financial data structures
//! into Polars `DataFrames` for advanced data analysis and manipulation.
//!
//! Run with: cargo run --example `14_polars_dataframes` --features dataframe

#[cfg(feature = "dataframe")]
use polars::prelude::*;

#[cfg(feature = "dataframe")]
use yfinance_rs::{Interval, Range, Ticker, YfClient};

#[cfg(feature = "dataframe")]
use yfinance_rs::{ToDataFrame, ToDataFrameVec};

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
    println!("💡 Tip: Use Polars' powerful DataFrame operations for advanced financial analysis!");
    println!("   - Filter data: df.filter(col(\"close\").gt(100))");
    println!("   - Sort data: df.sort([\"ts\"], Default::default())");
    println!("   - Group by: df.group_by([\"symbol\"]).agg([col(\"close\").mean()])");
    println!(
        "   - Join DataFrames: df1.join(&df2, [\"symbol\"], [\"symbol\"], JoinArgs::new(JoinType::Inner))"
    );

    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_history_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 1. Historical Price Data to DataFrame");
    let history = ticker
        .history(Some(Range::M6), Some(Interval::D1), false)
        .await?;

    if !history.is_empty() {
        println!("   Converting {} candles to DataFrame...", history.len());
        if let Ok(df) = history.to_dataframe() {
            println!("   DataFrame shape: {:?}", df.shape());
            println!("   Sample data:\n{}", df.head(Some(5)));
        }
    }
    println!();
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_quote_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 2. Current Quote to DataFrame");
    match ticker.quote().await {
        Ok(quote) => match quote.to_dataframe() {
            Ok(df) => {
                println!("   DataFrame shape: {:?}", df.shape());
                println!("   Quote data:\n{df}");
            }
            Err(e) => println!("   Error creating DataFrame: {e}"),
        },
        Err(e) => println!("   Error fetching quote: {e}"),
    }
    println!();
    Ok(())
}

#[cfg(feature = "dataframe")]
async fn section_recommendations_df(ticker: &Ticker) -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 3. Analyst Recommendations to DataFrame");
    match ticker.recommendations().await {
        Ok(recommendations) => {
            if recommendations.is_empty() {
                println!("   No recommendation data available");
            } else if let Ok(df) = recommendations.to_dataframe() {
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
            } else if let Ok(df) = financials.to_dataframe() {
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
        Ok(esg) => {
            println!("   Environmental: {:?}", esg.environmental);
            println!("   Social: {:?}", esg.social);
            println!("   Governance: {:?}", esg.governance);
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
            } else if let Ok(df) = holders.to_dataframe() {
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
    println!("🔍 7. Advanced Data Analysis Example");
    let history = ticker
        .history(Some(Range::M6), Some(Interval::D1), false)
        .await?;
    if history.is_empty() {
        return Ok(());
    }
    if let Ok(df) = history.to_dataframe() {
        let full_lf = df.lazy();
        let analysis_result = full_lf
            .clone()
            .select([
                col("close").mean().alias("avg_close"),
                col("close").min().alias("min_close"),
                col("close").max().alias("max_close"),
                col("volume").sum().alias("total_volume"),
                (col("high") - col("low")).mean().alias("avg_daily_range"),
            ])
            .collect();
        if let Ok(stats_df) = analysis_result {
            println!("   Price Analysis Results:");
            println!("{stats_df}");
        }
        let moving_avg_result = full_lf
            .sort(["ts"], SortMultipleOptions::default())
            .with_column(
                col("close")
                    .rolling_mean(RollingOptionsFixedWindow {
                        window_size: 5,
                        min_periods: 1,
                        ..Default::default()
                    })
                    .alias("5d_moving_avg"),
            )
            .select([col("ts"), col("close"), col("5d_moving_avg"), col("volume")])
            .limit(10)
            .collect();
        if let Ok(moving_avg_df) = moving_avg_result {
            println!("\n   Price Analysis (first 10 days with 5-day moving average):");
            println!("{moving_avg_df}");
        }
    }
    Ok(())
}

#[cfg(not(feature = "dataframe"))]
fn main() {
    println!("This example requires the 'dataframe' feature to be enabled.");
    println!("Run with: cargo run --example 14_polars_dataframes --features dataframe");
}
