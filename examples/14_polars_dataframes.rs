//! Example demonstrating Polars DataFrame integration with yfinance-rs.
//!
//! This example shows how to convert various financial data structures
//! into Polars DataFrames for advanced data analysis and manipulation.
//!
//! Run with: cargo run --example 14_polars_dataframes --features dataframe

#[cfg(feature = "dataframe")]
use polars::prelude::*;

#[cfg(feature = "dataframe")]
use yfinance_rs::{Interval, Range, Ticker, YfClient};

#[cfg(feature = "dataframe")]
use paft::dataframe::ToDataFrame;

#[cfg(feature = "dataframe")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();

    println!("=== Polars DataFrame Integration with yfinance-rs ===\n");

    // Example 1: Historical Price Data
    println!("📈 1. Historical Price Data to DataFrame");
    let ticker = Ticker::new(&client, "AAPL");
    let history = ticker
        .history(Some(Range::M6), Some(Interval::D1), false)
        .await?;

    if !history.is_empty() {
        println!("   Converting {} candles to DataFrame...", history.len());
        match history.to_dataframe() {
            Ok(df) => {
                println!("   DataFrame shape: {:?}", df.shape());
                println!("   Sample data:\n{}", df.head(Some(5)));
            }
            Err(e) => println!("   Error creating DataFrame: {}", e),
        }
    }

    println!();

    // Example 2: Quote Data
    println!("📊 2. Current Quote to DataFrame");
    match ticker.quote().await {
        Ok(quote) => {
            println!("   Converting quote data to DataFrame...");
            match quote.to_dataframe() {
                Ok(df) => {
                    println!("   DataFrame shape: {:?}", df.shape());
                    println!("   Quote data:\n{}", df);
                }
                Err(e) => println!("   Error creating DataFrame: {}", e),
            }
        }
        Err(e) => println!("   Error fetching quote: {}", e),
    }

    println!();

    // Example 3: Analyst Recommendations
    println!("📈 3. Analyst Recommendations to DataFrame");
    match ticker.recommendations().await {
        Ok(recommendations) => {
            if !recommendations.is_empty() {
                println!(
                    "   Converting {} recommendation periods to DataFrame...",
                    recommendations.len()
                );
                match recommendations.to_dataframe() {
                    Ok(df) => {
                        println!("   DataFrame shape: {:?}", df.shape());
                        println!("   Recommendation data:\n{}", df.head(Some(5)));
                    }
                    Err(e) => println!("   Error creating DataFrame: {}", e),
                }
            } else {
                println!("   No recommendation data available");
            }
        }
        Err(e) => println!("   Error fetching recommendations: {}", e),
    }

    println!();

    // Example 4: Financial Fundamentals
    println!("💰 4. Financial Statements to DataFrame");
    match ticker.income_stmt().await {
        Ok(financials) => {
            if !financials.is_empty() {
                println!(
                    "   Converting {} annual income statements to DataFrame...",
                    financials.len()
                );
                match financials.to_dataframe() {
                    Ok(df) => {
                        println!("   DataFrame shape: {:?}", df.shape());
                        println!("   Income statement data:\n{}", df.head(Some(3)));
                    }
                    Err(e) => println!("   Error creating DataFrame: {}", e),
                }
            } else {
                println!("   No financial data available");
            }
        }
        Err(e) => println!("   Error fetching financials: {}", e),
    }

    println!();

    // Example 5: ESG Data
    println!("🌱 5. ESG Involvement Data to DataFrame");
    match ticker.sustainability().await {
        Ok(esg) => {
            println!("   Converting ESG involvement flags to DataFrame...");

            match esg.involvement.to_dataframe() {
                Ok(df) => {
                    println!("   DataFrame shape: {:?}", df.shape());
                    println!("   ESG involvement data:\n{}", df);
                }
                Err(e) => println!("   Error creating DataFrame: {}", e),
            }
        }
        Err(e) => println!("   ESG data not available for this ticker: {}", e),
    }

    println!();

    // Example 6: Institutional Holders
    println!("🏛️ 6. Institutional Holders to DataFrame");
    match ticker.institutional_holders().await {
        Ok(holders) => {
            if !holders.is_empty() {
                println!(
                    "   Converting {} institutional holders to DataFrame...",
                    holders.len()
                );
                match holders.to_dataframe() {
                    Ok(df) => {
                        println!("   DataFrame shape: {:?}", df.shape());
                        println!("   Top institutional holders:\n{}", df.head(Some(5)));
                    }
                    Err(e) => println!("   Error creating DataFrame: {}", e),
                }
            } else {
                println!("   No institutional holders data available");
            }
        }
        Err(e) => println!("   Institutional holders data not available: {}", e),
    }

    println!();

    // Example 7: Data Analysis with Polars
    println!("🔍 7. Advanced Data Analysis Example");

    // Let's analyze the historical data using Polars operations
    if !history.is_empty() {
        println!("   Performing advanced analysis on price data...");

        if let Ok(df) = history.to_dataframe() {
            let full_lf = df.lazy();

            // Calculate some basic statistics
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

            match analysis_result {
                Ok(stats_df) => {
                    println!("   Price Analysis Results:");
                    println!("{}", stats_df);
                }
                Err(e) => println!("   Error in analysis: {}", e),
            }

            // Calculate a 5-period moving average.
            // Since the data interval is daily, this is a 5-day moving average.
            let moving_avg_result = full_lf
                .sort(["ts"], Default::default()) // Sorting is important for window functions
                .with_column(
                    col("close")
                        .rolling_mean(RollingOptionsFixedWindow {
                            window_size: 5,
                            min_periods: 1, // Start calculating even if the window is not full
                            ..Default::default()
                        })
                        .alias("5d_moving_avg"),
                )
                .select([col("ts"), col("close"), col("5d_moving_avg"), col("volume")])
                .limit(10)
                .collect();

            match moving_avg_result {
                Ok(moving_avg_df) => {
                    println!("\n   Price Analysis (first 10 days with 5-day moving average):");
                    println!("{}", moving_avg_df);
                }
                Err(e) => println!("   Error calculating analysis: {}", e),
            }
        }
    }

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

#[cfg(not(feature = "dataframe"))]
fn main() {
    println!("This example requires the 'dataframe' feature to be enabled.");
    println!("Run with: cargo run --example 14_polars_dataframes --features dataframe");
}
