
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

* **Historical Data**: Fetch daily, weekly, or monthly OHLCV data for any ticker.
* **Real-time Streaming**: Get live quote updates using WebSockets (with an HTTP polling fallback).
* **Company Profiles**: Retrieve detailed information about companies and funds.
* **Options Chains**: Fetch expiration dates and full option chains (calls and puts).
* **Financials**: Access income statements, balance sheets, and cash flow statements (annual & quarterly).
* **Analyst Ratings**: Get price targets, recommendations, and upgrade/downgrade history.
* **Async API**: Built on `tokio` and `reqwest` for non-blocking I/O.
* **High-Level `Ticker` Interface**: A convenient, yfinance-like struct for accessing all data for a single symbol.
* **Builder Pattern**: Fluent builders for constructing complex queries.
* **In-memory Caching**: Optional caching to reduce redundant network requests.
* **Configurable Retries**: Automatic retries with exponential backoff for transient network errors.

## Quick Start

To get started, add `yfinance-rs` to your `Cargo.toml`:

```toml
[dependencies]
yfinance-rs = "0.0.1"
tokio = { version = "1", features = ["full"] }
```

Then, create a `YfClient` and use a `Ticker` to fetch data.

```rust
use yfinance_rs::{Interval, Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::new();
    let ticker = Ticker::new(client, "AAPL".to_string())?;

    // Get the latest quote
    let quote = ticker.quote().await?;
    println!("Latest price for AAPL: ${:.2}", quote.regular_market_price.unwrap_or(0.0));

    // Get historical data for the last 6 months
    let history = ticker.history(None, Some(Interval::D1), false).await?;
    if let some(last_bar) = history.last() {
        println!("Last closing price: ${:.2} on timestamp {}", last_bar.close, last_bar.ts);
    }

    // Get analyst recommendations
    let recs = ticker.recommendations().await?;
    if let some(latest_rec) = recs.first() {
        println!("Latest recommendation period: {}", latest_rec.period);
    }

    Ok(())
}
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.
