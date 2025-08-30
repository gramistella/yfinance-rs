# yfinance-rs

An ergonomic, async-first Rust client for the unofficial Yahoo Finance API.

This crate provides a simple and efficient way to fetch financial data from Yahoo Finance.
It is designed to feel familiar to users of the popular Python `yfinance` library, but
leverages Rust's powerful type system and async capabilities for performance and safety.

[![Crates.io](https://img.shields.io/crates/v/yfinance-rs.svg)](https://crates.io/crates/yfinance-rs)
[![Docs.rs](https://docs.rs/yfinance-rs/badge.svg)](https://docs.rs/yfinance-rs)
[![CI](https://github.com/gramistella/yfinance-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/gramistella/yfinance-rs/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/yfinance-rs)](https://crates.io/crates/yfinance-rs)
[![License](https://img.shields.io/crates/l/yfinance-rs)](https://github.com/gramistella/yfinance-rs/blob/main/LICENSE)

## Features

### Core Data
* **Historical Data**: Fetch daily, weekly, or monthly OHLCV data with automatic split/dividend adjustments.
* **Real-time Quotes**: Get live quote updates with detailed market information.
* **Fast Quotes**: Optimized quote fetching with essential data only (`fast_info`).
* **Multi-Symbol Downloads**: Concurrently download historical data for many symbols at once.
* **Batch Quotes**: Fetch quotes for multiple symbols efficiently.

### Corporate Actions & Dividends
* **Dividend History**: Fetch complete dividend payment history with amounts and dates.
* **Stock Splits**: Get stock split history with split ratios.
* **Capital Gains**: Retrieve capital gains distributions (especially for mutual funds).
* **All Corporate Actions**: Comprehensive access to dividends, splits, and capital gains in one call.

### Financial Statements & Fundamentals
* **Income Statements**: Access annual and quarterly income statements.
* **Balance Sheets**: Get annual and quarterly balance sheet data.
* **Cash Flow Statements**: Fetch annual and quarterly cash flow data.
* **Earnings Data**: Historical earnings, revenue estimates, and EPS data.
* **Shares Outstanding**: Historical data on shares outstanding (annual and quarterly).
* **Corporate Calendar**: Earnings dates, ex-dividend dates, and dividend payment dates.

### Options & Derivatives
* **Options Chains**: Fetch expiration dates and full option chains (calls and puts).
* **Option Contracts**: Detailed option contract information including Greeks.

### Analysis & Research
* **Analyst Ratings**: Get price targets, recommendations, and upgrade/downgrade history.
* **Earnings Trends**: Detailed earnings and revenue estimates from analysts.
* **Recommendations Summary**: Summary of current analyst recommendations.
* **Upgrades/Downgrades**: History of analyst rating changes.

### Ownership & Holders
* **Major Holders**: Get major, institutional, and mutual fund holder data.
* **Institutional Holders**: Top institutional shareholders and their holdings.
* **Mutual Fund Holders**: Mutual fund ownership breakdown.
* **Insider Transactions**: Recent insider buying and selling activity.
* **Insider Roster**: Company insiders and their current holdings.
* **Net Share Activity**: Summary of insider purchase/sale activity.

### ESG & Sustainability
* **ESG Scores**: Fetch detailed Environmental, Social, and Governance ratings.
* **ESG Involvement**: Specific ESG involvement and controversy data.

### News & Information
* **Company News**: Retrieve the latest articles and press releases for a ticker.
* **Company Profiles**: Detailed information about companies, ETFs, and funds.
* **Search**: Find tickers by name or keyword.

### Real-time Streaming
* **WebSocket Streaming**: Get live quote updates using WebSockets (preferred method).
* **HTTP Polling**: Fallback polling method for real-time data.
* **Configurable Streaming**: Customize update frequency and change-only filtering.

### Advanced Features
* **Data Repair**: Automatic detection and repair of price outliers.
* **Data Rounding**: Control price precision and rounding.
* **Missing Data Handling**: Configurable handling of NA/missing values.
* **Back Adjustment**: Alternative price adjustment methods.
* **Historical Metadata**: Timezone and other metadata for historical data.
* **ISIN Lookup**: Get International Securities Identification Numbers.

### Developer Experience
* **Async API**: Built on `tokio` and `reqwest` for non-blocking I/O.
* **High-Level `Ticker` Interface**: A convenient, yfinance-like struct for accessing all data for a single symbol.
* **Builder Pattern**: Fluent builders for constructing complex queries.
* **Configurable Retries**: Automatic retries with exponential backoff for transient network errors.
* **Caching**: Configurable caching behavior for API responses.
* **Custom Timeouts**: Configurable request timeouts and connection settings.

## Quick Start

To get started, add `yfinance-rs` to your `Cargo.toml`:

```toml
[dependencies]
yfinance-rs = "0.1.2"
tokio = { version = "1", features = ["full"] }
```

Then, create a `YfClient` and use a `Ticker` to fetch data.

```rust
use yfinance_rs::{Interval, Range, Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(client, "AAPL".to_string());

    // Get the latest quote
    let quote = ticker.quote().await?;
    println!("Latest price for AAPL: ${:.2}", quote.regular_market_price.unwrap_or(0.0));

    // Get historical data for the last 6 months
    let history = ticker.history(Some(Range::M6), Some(Interval::D1), false).await?;
    if let Some(last_bar) = history.last() {
        println!("Last closing price: ${:.2} on timestamp {}", last_bar.close, last_bar.ts);
    }

    // Get analyst recommendations
    let recs = ticker.recommendations().await?;
    if let Some(latest_rec) = recs.first() {
        println!("Latest recommendation period: {}", latest_rec.period);
    }

    // Get dividend history
    let dividends = ticker.dividends(Some(Range::Y1)).await?;
    println!("Found {} dividend payments in the last year", dividends.len());

    // Get earnings trends
    let trends = ticker.earnings_trend().await?;
    if let Some(latest_trend) = trends.first() {
        println!("Latest earnings estimate: ${:.2}", latest_trend.eps_estimate.unwrap_or(0.0));
    }

    Ok(())
}
```

## Advanced Examples

### Multi-Symbol Data Download

```rust
use yfinance_rs::{DownloadBuilder, Interval};
use chrono::{Duration, Utc};

let client = YfClient::default();
let symbols = vec!["AAPL", "GOOGL", "MSFT", "TSLA"];

let results = DownloadBuilder::new(&client)
    .symbols(symbols)
    .range(Range::M6)
    .interval(Interval::D1)
    .auto_adjust(true)
    .actions(true)
    .repair(true)  // Fix price outliers
    .rounding(true)  // Round to 2 decimal places
    .run()
    .await?;

for (symbol, candles) in &results.series {
    println!("{}: {} data points", symbol, candles.len());
}
```

### Real-time Streaming

```rust
use yfinance_rs::{StreamBuilder, StreamMethod, StreamConfig};
use std::time::Duration;

let config = StreamConfig {
    interval: Duration::from_secs(1),
    diff_only: true,  // Only emit updates when price changes
};

let (handle, mut receiver) = StreamBuilder::new(&client)
    .symbols(vec!["AAPL", "GOOGL"])
    .method(StreamMethod::WebsocketWithFallback)
    .config(config)
    .start()?;

// Process updates
while let Some(update) = receiver.recv().await {
    println!("{}: ${:.2}", update.symbol, update.last_price.unwrap_or(0.0));
}

// Stop the stream
handle.stop().await;
```

### Financial Statements

```rust
let ticker = Ticker::new(client, "AAPL");

// Get quarterly financials
let income_stmt = ticker.quarterly_income_stmt().await?;
let balance_sheet = ticker.quarterly_balance_sheet().await?;
let cashflow = ticker.quarterly_cashflow().await?;

// Get shares outstanding
let shares = ticker.quarterly_shares().await?;
if let Some(latest) = shares.first() {
    println!("Latest shares outstanding: {}", latest.shares);
}
```

### Options Trading

```rust
let ticker = Ticker::new(client, "AAPL");

// Get available expiration dates
let expirations = ticker.options().await?;

// Get option chain for nearest expiration
if let Some(nearest) = expirations.first() {
    let chain = ticker.option_chain(Some(*nearest)).await?;
    
    println!("Calls: {}", chain.calls.len());
    println!("Puts: {}", chain.puts.len());
    
    // Find ATM options
    let current_price = ticker.fast_info().await?.last_price;
    for call in &chain.calls {
        if (call.strike - current_price).abs() < 5.0 {
            println!("ATM Call: Strike ${:.2}, Bid ${:.2}, Ask ${:.2}", 
                     call.strike, call.bid.unwrap_or(0.0), call.ask.unwrap_or(0.0));
        }
    }
}
```

### Advanced Analysis

```rust
let ticker = Ticker::new(client, "AAPL");

// Get comprehensive analyst data
let price_target = ticker.analyst_price_target().await?;
let recs_summary = ticker.recommendations_summary().await?;
let upgrades = ticker.upgrades_downgrades().await?;
let earnings_trends = ticker.earnings_trend().await?;

println!("Price Target: ${:.2}", price_target.target_mean_price.unwrap_or(0.0));
println!("Recommendation: {}", recs_summary.recommendation_mean.unwrap_or_default());
```

### Holder Information

```rust
let ticker = Ticker::new(client, "AAPL");

// Get ownership breakdown
let major_holders = ticker.major_holders().await?;
let institutional = ticker.institutional_holders().await?;
let mutual_funds = ticker.mutual_fund_holders().await?;
let insider_transactions = ticker.insider_transactions().await?;

for holder in &major_holders {
    println!("{}: {:.1}%", holder.holder, holder.percent);
}
```

### ESG & Sustainability

```rust
let ticker = Ticker::new(client, "AAPL");

let esg = ticker.sustainability().await?;
println!("Total ESG Score: {:.2}", esg.total_esg_score.unwrap_or(0.0));
println!("Environmental Score: {:.2}", esg.environment_score.unwrap_or(0.0));
println!("Social Score: {:.2}", esg.social_score.unwrap_or(0.0));
println!("Governance Score: {:.2}", esg.governance_score.unwrap_or(0.0));
```

### Advanced Client Configuration

```rust
use yfinance_rs::{YfClientBuilder, CacheMode, RetryConfig};
use std::time::Duration;

let client = YfClientBuilder::default()
    .timeout(Duration::from_secs(10))
    .retry_config(RetryConfig {
        max_retries: 3,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        ..Default::default()
    })
    .build()?;

// Use with custom cache settings
let ticker = Ticker::new(client, "AAPL")
    .cache_mode(CacheMode::Bypass)  // Skip caching for this ticker
    .retry_policy(Some(RetryConfig {
        max_retries: 5,
        ..Default::default()
    }));
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.