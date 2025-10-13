# yfinance-rs

[![Crates.io](https://img.shields.io/crates/v/yfinance-rs.svg)](https://crates.io/crates/yfinance-rs)
[![Docs.rs](https://docs.rs/yfinance-rs/badge.svg)](https://docs.rs/yfinance-rs)
[![CI](https://github.com/gramistella/yfinance-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/gramistella/yfinance-rs/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/yfinance-rs)](https://crates.io/crates/yfinance-rs)
[![License](https://img.shields.io/crates/l/yfinance-rs)](LICENSE)

## Overview

An ergonomic, async-first Rust client for the unofficial Yahoo Finance API. It provides a simple and efficient way to fetch financial data, with a convenient, yfinance-like API, leveraging Rust's type system and async runtime for performance and safety.

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
* **Option Contracts**: Detailed option contract information.

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

### Real-time Streaming (WebSocket/Polling)

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
* **Polars DataFrames**: Convert results to Polars DataFrames via `.to_dataframe()` (enable the `dataframe` feature).

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
yfinance-rs = "0.4.0"
tokio = { version = "1", features = ["full"] }
```

To enable DataFrame conversions backed by Polars, turn on the optional `dataframe` feature and (if you use Polars types in your code) add `polars`:

```toml
[dependencies]
yfinance-rs = { version = "0.4.0", features = ["dataframe"] }
polars = "0.51"
```

Then, create a `YfClient` and use a `Ticker` to fetch data.

```rust
use yfinance_rs::{Interval, Range, Ticker, YfClient};
use yfinance_rs::core::conversions::money_to_f64;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    // Get the latest quote
    let quote = ticker.quote().await?;
    println!(
        "Latest price for AAPL: ${:.2}",
        quote.price.as_ref().map(money_to_f64).unwrap_or(0.0)
    );

    // Get historical data for the last 6 months
    let history = ticker.history(Some(Range::M6), Some(Interval::D1), false).await?;
    if let Some(last_bar) = history.last() {
        println!(
            "Last closing price: ${:.2} on {}",
            money_to_f64(&last_bar.close),
            last_bar.ts
        );
    }

    // Get analyst recommendations
    let recs = ticker.recommendations().await?;
    if let Some(latest_rec) = recs.first() {
        println!("Latest recommendation period: {}", latest_rec.period);
    }

    // Dividends in the last year
    let dividends = ticker.dividends(Some(Range::Y1)).await?;
    println!("Found {} dividend payments in the last year", dividends.len());

    // Earnings trend
    let trends = ticker.earnings_trend(None).await?;
    if let Some(latest) = trends.first() {
        println!(
            "Latest earnings estimate: ${:.2}",
            latest
                .earnings_estimate
                .avg
                .as_ref()
                .map(money_to_f64)
                .unwrap_or(0.0)
        );
    }

    Ok(())
}
```

### Troubleshooting

**Possible network or consent issues**

Some users [have reported](https://github.com/gramistella/yfinance-rs/issues/1) encountering errors on first use, such as:

- `Rate limited at ...`  
- `HTTP error: error sending request for url (https://fc.yahoo.com/consent)`

These are typically **environmental** (network or regional) issues with Yahoo’s public API.  
In some regions, Yahoo may require a one-time consent or session initialization.

**Workaround:**  
Open [`https://fc.yahoo.com/consent`](https://fc.yahoo.com/consent) in a web browser **from the same network** before running your code again.  
This usually resolves the issue for that IP/network.

## Advanced Examples

### Polars DataFrames (to_dataframe)

Enable the `dataframe` feature to convert paft models into a Polars `DataFrame` with `.to_dataframe()`.

```rust
use yfinance_rs::{Interval, Range, Ticker, YfClient};
use paft::prelude::{ToDataFrame, ToDataFrameVec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    // Quote → DataFrame
    let quote_df = ticker.quote().await?.to_dataframe()?;
    println!("Quote as DataFrame:\n{}", quote_df);

    // History (Vec<Candle>) → DataFrame
    let hist_df = ticker
        .history(Some(Range::M1), Some(Interval::D1), false)
        .await?
        .to_dataframe()?;
    println!("History rows: {}", hist_df.height());

    Ok(())
}
```

Works for quotes, historical candles, fundamentals, analyst data, holders, options, and more. All `paft` structs returned by this crate implement `.to_dataframe()` when the `dataframe` feature is enabled. See the full example: `examples/14_polars_dataframes.rs`.

### Multi-Symbol Data Download

```rust
use yfinance_rs::{DownloadBuilder, Interval, Range, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let symbols = vec!["AAPL", "GOOGL", "MSFT", "TSLA"];

    let results = DownloadBuilder::new(&client)
        .symbols(symbols)
        .range(Range::M6)
        .interval(Interval::D1)
        .auto_adjust(true)
        .actions(true)
        .repair(true)
        .rounding(true)
        .run()
        .await?;

    for (symbol, candles) in &results.series {
        println!("{}: {} data points", symbol, candles.len());
    }
    Ok(())
}
```

### Real-time Streaming

```rust
use yfinance_rs::{StreamBuilder, StreamMethod, YfClient};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let (handle, mut receiver) = StreamBuilder::new(&client)
        .symbols(vec!["AAPL", "GOOGL"]) 
        .method(StreamMethod::WebsocketWithFallback)
        .interval(Duration::from_secs(1))
        .diff_only(true)
        .start()?;

    while let Some(update) = receiver.recv().await {
        println!("{}: ${:.2}", update.symbol, update.last_price.unwrap_or(0.0));
    }

    Ok(())
}
```

### Financial Statements

```rust
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    let income_stmt = ticker.quarterly_income_stmt(None).await?;
    let balance_sheet = ticker.quarterly_balance_sheet(None).await?;
    let cashflow = ticker.quarterly_cashflow(None).await?;

    println!("Found {} quarterly income statements.", income_stmt.len());
    println!("Found {} quarterly balance sheet statements.", balance_sheet.len());
    println!("Found {} quarterly cashflow statements.", cashflow.len());

    let shares = ticker.quarterly_shares().await?;
    if let Some(latest) = shares.first() {
        println!("Latest shares outstanding: {}", latest.shares); 
    }
    Ok(())
}
```

> 💡 Need to force a specific reporting currency? Pass `Some(paft::money::Currency::USD)` (or another currency) instead of `None` when calling the fundamentals/analysis helpers.

### Options Trading

```rust
use yfinance_rs::{Ticker, YfClient};
use yfinance_rs::core::conversions::money_to_f64;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    let expirations = ticker.options().await?;

    if let Some(nearest) = expirations.first() {
        let chain = ticker.option_chain(Some(*nearest)).await?;
    
        println!("Calls: {}", chain.calls.len());
        println!("Puts: {}", chain.puts.len());
    
        let fi = ticker.fast_info().await?;
        let current_price = fi
            .last
            .as_ref()
            .map(money_to_f64)
            .or_else(|| fi.previous_close.as_ref().map(money_to_f64))
            .unwrap_or(0.0);
        for call in &chain.calls {
            if (money_to_f64(&call.strike) - current_price).abs() < 5.0 {
                println!(
                    "ATM Call: Strike ${:.2}, Bid ${:.2}, Ask ${:.2}", 
                    money_to_f64(&call.strike),
                    call.bid.as_ref().map(money_to_f64).unwrap_or(0.0),
                    call.ask.as_ref().map(money_to_f64).unwrap_or(0.0)
                );
            }
        }
    }
    Ok(())
}
```

### Advanced Analysis

```rust
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    let price_target = ticker.analyst_price_target(None).await?;
    let recs_summary = ticker.recommendations_summary().await?;
    let upgrades = ticker.upgrades_downgrades().await?;
    let earnings_trends = ticker.earnings_trend(None).await?;

    println!(
        "Price Target: ${:.2}",
        price_target.mean.as_ref().map(yfinance_rs::core::conversions::money_to_f64).unwrap_or(0.0)
    );
    println!(
        "Recommendation: {}",
        recs_summary
            .mean_rating_text
            .as_deref()
            .unwrap_or("N/A")
    );
    println!("Trend rows: {}", earnings_trends.len());
    println!("Upgrades: {}", upgrades.len());

    Ok(())
}
```

### Holder Information

```rust
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    let major_holders = ticker.major_holders().await?;
    let institutional = ticker.institutional_holders().await?;
    let mutual_funds = ticker.mutual_fund_holders().await?;
    let insider_transactions = ticker.insider_transactions().await?;

    for holder in &major_holders {
        println!("{}: {}", holder.category, holder.value);
    }
    println!("Institutional rows: {}", institutional.len());
    println!("Mutual fund rows: {}", mutual_funds.len());
    println!("Insider transactions: {}", insider_transactions.len());
    Ok(())
}
```

### ESG Scores & Involvement

```rust
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    let summary = ticker.sustainability().await?;
    let parts = summary
        .scores
        .as_ref()
        .map(|s| [s.environmental, s.social, s.governance])
        .unwrap_or([None, None, None]);
    let vals = parts.into_iter().flatten().collect::<Vec<_>>();
    let total = if vals.is_empty() { 0.0 } else { vals.iter().copied().sum::<f64>() / (vals.len() as f64) };
    println!("Total ESG Score: {:.2}", total);
    if let Some(scores) = summary.scores.as_ref() {
        println!("Environmental Score: {:.2}", scores.environmental.unwrap_or(0.0));
        println!("Social Score: {:.2}", scores.social.unwrap_or(0.0));
        println!("Governance Score: {:.2}", scores.governance.unwrap_or(0.0));
    }
    Ok(())
}
```

### Advanced Client Configuration

```rust
use yfinance_rs::{YfClientBuilder, Ticker, core::client::{Backoff, CacheMode, RetryConfig}};
use std::time::Duration;
use yfinance_rs::core::conversions::money_to_f64;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClientBuilder::default()
        .timeout(Duration::from_secs(10))
        .retry_config(RetryConfig {
            max_retries: 3,
            backoff: Backoff::Exponential {
                base: Duration::from_millis(100),
                factor: 2.0,
                max: Duration::from_secs(5),
                jitter: true,
            },
            ..Default::default()
        })
        .build()?;

    let ticker = Ticker::new(&client, "AAPL")
        .cache_mode(CacheMode::Bypass)
        .retry_policy(Some(RetryConfig {
            max_retries: 5,
            ..Default::default()
        }));
    
    let quote = ticker.quote().await?;
    println!(
        "Latest price for AAPL with custom client: ${:.2}",
        quote.price.as_ref().map(money_to_f64).unwrap_or(0.0)
    );

    Ok(())
}

```

#### Custom Reqwest Client

For full control over HTTP configuration, you can provide your own reqwest client:

```rust
use yfinance_rs::{YfClient, Ticker};
use yfinance_rs::core::conversions::money_to_f64;
use reqwest::Client;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let custom_client = Client::builder()
        .user_agent("yfinance-rs-playground") // Make sure to set a proper user agent
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .build()?;

    let client = YfClient::builder()
        .custom_client(custom_client)
        .cache_ttl(Duration::from_secs(300))
        .build()?;

    let ticker = Ticker::new(&client, "AAPL");
    let quote = ticker.quote().await?;
    println!(
        "Latest price for AAPL: ${:.2}",
        quote.price.as_ref().map(money_to_f64).unwrap_or(0.0)
    );

    Ok(())
}
```

#### Proxy Configuration

You can configure HTTP/HTTPS proxies through the builder:

```rust
use yfinance_rs::{YfClient, YfClientBuilder, Ticker};
use yfinance_rs::core::conversions::money_to_f64;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::builder()
        .try_proxy("http://proxy.example.com:8080")?
        .timeout(Duration::from_secs(30))
        .build()?;

    let client_https = YfClient::builder()
        .try_https_proxy("https://proxy.example.com:8443")?
        .timeout(Duration::from_secs(30))
        .build()?;

    let client_simple = YfClient::builder()
        .proxy("http://proxy.example.com:8080")
        .timeout(Duration::from_secs(30))
        .build()?;

    let ticker = Ticker::new(&client, "AAPL");
    let quote = ticker.quote().await?;
    println!(
        "Latest price for AAPL via proxy: ${:.2}",
        quote.price.as_ref().map(money_to_f64).unwrap_or(0.0)
    );

    Ok(())
}
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Please see our [Contributing Guide](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md). We welcome pull requests and issues.

## Changelog

See **[CHANGELOG.md](https://github.com/gramistella/yfinance-rs/blob/main/CHANGELOG.md)** for release notes and breaking changes.
