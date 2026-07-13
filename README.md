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

* **Typed Corporate Actions**: Fetch dividends, splits, and capital gains through one currency-aware action stream.

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

* **ESG Scores**: Fetch Environmental, Social, and Governance ratings when Yahoo returns them.
* **ESG Involvement**: Specific ESG involvement and controversy data when Yahoo returns it.

### News & Information

* **Company News**: Retrieve the latest articles and press releases for a ticker.
* **Company Profiles**: Detailed information about companies, ETFs, and funds.
* **Search**: Find tickers by name or keyword.

### Real-time Streaming (WebSocket/Polling)

Streaming is behind the `stream` feature:

```toml
yfinance-rs = { version = "0.10.0", features = ["stream"] }
```

* **WebSocket Streaming**: Get live quote updates using WebSockets (preferred method).
* **HTTP Polling**: Fallback polling method for real-time data.
* **Configurable Streaming**: Customize update frequency and change-only filtering.
* **Cumulative stream volume**: `QuoteUpdate.volume` reflects Yahoo's latest cumulative session volume when Yahoo sends it.

### Advanced Features

* **Data Rounding**: Control price precision and rounding.
* **Malformed Data Handling**: Drops invalid OHLC rows while preserving valid sibling data.
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

## Cargo Features

User-facing optional features:

- `stream`: enables WebSocket streaming support.
- `dataframe`: re-exports `paft` DataFrame conversion traits.
- `tracing`: compiles structured tracing instrumentation.

Internal/unstable features:

- `test-mode`: enables repository test hooks, fixture recording, and doc-hidden plumbing APIs for yfinance-rs' own integration tests.
- `debug-dumps`: enables maintainer diagnostics for dumping selected raw Yahoo responses.
- `tracing-subscriber`: test/example convenience for initializing a basic subscriber; applications should configure their own subscriber instead.

Internal features are published only because Cargo has no private feature namespace. They are not part of the supported user-facing API.

## Quick Start

To get started, add `yfinance-rs` to your `Cargo.toml`:

```toml
[dependencies]
yfinance-rs = "0.10.0"
tokio = { version = "1", features = ["full"] }
```

To enable DataFrame conversions backed by Polars, turn on the optional `dataframe` feature and (if you use Polars types in your code) add `polars`:

```toml
[dependencies]
yfinance-rs = { version = "0.10.0", features = ["dataframe"] }
polars = "0.53"
```

Then, create a `YfClient` and use a `Ticker` to fetch data.

```rust
use yfinance_rs::{Action, Interval, Range, Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    // Get the latest quote
    let quote = ticker.quote().await?;
    if let Some(price) = quote.price.as_ref() {
        println!("Latest price for AAPL: {price}");
    }

    // Get historical data for the last 6 months
    let history = ticker.history(Some(Range::M6), Some(Interval::D1), false).await?;
    if let Some(last_bar) = history.last() {
        println!(
            "Last closing price: {} on {}",
            last_bar.ohlc.close, last_bar.ts
        );
    }

    // Get analyst recommendations
    let recs = ticker.recommendations().await?;
    if let Some(latest_rec) = recs.first() {
        println!("Latest recommendation period: {}", latest_rec.period);
    }

    // Dividends in the last year
    let actions = ticker.actions(Some(Range::Y1)).await?;
    let dividends = actions
        .iter()
        .filter(|action| matches!(action, Action::Dividend { .. }))
        .count();
    println!("Found {dividends} dividend payments in the last year");

    // Earnings trend
    let trends = ticker.earnings_trend(None).await?;
    if let Some(latest) = trends.first() {
        if let Some(avg) = latest.earnings_estimate.avg.as_ref() {
            println!("Latest earnings estimate: {avg}");
        }
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

### Tracing (optional)

This crate can emit structured tracing spans and key events when the optional `tracing` feature is enabled. When disabled (default), all instrumentation is compiled out with zero overhead. The library does not configure a subscriber; set one up in your application.

Spans are added at: `Ticker` public APIs (`info`, `quote`, `history`, etc.), HTTP `send_with_retry`, quote summary fetch (including invalid-crumb retry), and full history fetch. Key events include retry/backoff, optional module failures, stream decode/fallback diagnostics, currency resolution cache updates, and test fixture/debug dump writes.

## Advanced Examples

### Yahoo Screeners

Use predefined Yahoo screeners or build strongly typed custom equity, ETF, and fund queries.

```rust
use yfinance_rs::{
    Decimal, EquityQuery, PercentPoints, PredefinedScreener, Region, ScreenerBuilder, YfClient,
    equity_fields, screen,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();

    let gainers = screen(&client, PredefinedScreener::DayGainers).await?;
    println!("day gainers: {}", gainers.results.len());

    let query = EquityQuery::and(vec![
        equity_fields::REGION.eq(Region::Us),
        equity_fields::INTRADAY_PRICE.gte(5),
        equity_fields::PERCENT_CHANGE.gt(PercentPoints::new(Decimal::new(20, 1))),
    ])?;

    let custom = ScreenerBuilder::equity(&client, query).fetch().await?;
    println!("custom screen: {}", custom.results.len());

    Ok(())
}
```

See the full example: `examples/15_screeners.rs`.

### Polars DataFrames (to_dataframe)

Enable the `dataframe` feature to convert returned models into a Polars `DataFrame` with `.to_dataframe()`.

```rust
use yfinance_rs::{Interval, Range, Ticker, ToDataFrame, ToDataFrameVec, YfClient};

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

Works for quotes, historical candles, fundamentals, analyst data, holders, options, and more. Returned model structs implement `.to_dataframe()` when the `dataframe` feature is enabled. See the full example: `examples/14_polars_dataframes.rs`.

### Multi-Symbol Data Download

```rust
use yfinance_rs::{DownloadAdjustment, DownloadBuilder, Interval, Range, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let symbols = vec!["AAPL", "GOOGL", "MSFT", "TSLA"];

    let results = DownloadBuilder::new(&client)
        .symbols(symbols)
        .range(Range::M6)
        .interval(Interval::D1)
        .adjustment(DownloadAdjustment::Auto)
        .actions(true)
        .rounding(true)
        .run()
        .await?;

    for entry in &results.entries {
        println!(
            "{}: {} data points",
            entry.instrument.symbol.as_str(),
            entry.history.candles.len()
        );
    }
    Ok(())
}
```

For back-adjusted downloads, use `.back_adjust()` or
`.adjustment(DownloadAdjustment::Back)`.

### Real-time Streaming

Enable the `stream` feature to use this API:

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
        .start()
        .await?;

    while let Some(update) = receiver.recv().await {
        let vol = update.volume.map(|v| format!(" (volume: {v})")).unwrap_or_default();
        let price = update
            .price
            .as_ref()
            .map_or_else(|| "N/A".to_string(), ToString::to_string);
        println!("{}: {}{}", update.instrument, price, vol);
    }

    Ok(())
}
```

#### Volume semantics

Yahoo’s websocket stream provides cumulative intraday volume (`day_volume`), and v7 quote polling provides the same concept as `regularMarketVolume`. `QuoteUpdate::volume` exposes the latest cumulative value directly:

- WebSocket stream: maps Yahoo `day_volume` to `volume`.
- Polling stream: maps Yahoo `regularMarketVolume` to `volume`.
- No per-symbol volume state is kept inside the stream.
- `diff_only(true)` filters on price changes; volume-only changes do not emit a new polling update.

If you need deltas, day-boundary handling, or provider-adjustment detection, derive those from successive cumulative values in application code.

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

`ticker.shares()` and `ticker.quarterly_shares()` use Yahoo's rolling 548-day
share-count window, matching Python yfinance's `get_shares_full(start=None,
end=None)` default. Use `shares_between(start, end)` or
`quarterly_shares_between(start, end)` when you need older points.

> 💡 Need to force a specific reporting currency? Import `Currency` and `IsoCurrency` from `yfinance_rs`, then pass `Some(Currency::Iso(IsoCurrency::USD))` (or another currency) instead of `None` when calling the fundamentals/analysis helpers.

### Options Trading

```rust
use yfinance_rs::{Ticker, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    let expirations = ticker.options().await?;

    if let Some(nearest) = expirations.first() {
        let chain = ticker.option_chain(Some(*nearest)).await?;

        let calls = chain.calls().collect::<Vec<_>>();
        let puts = chain.puts().collect::<Vec<_>>();

        println!("Calls: {}", calls.len());
        println!("Puts: {}", puts.len());

        for call in calls.iter().take(5) {
            println!(
                "Call: Strike {}, Bid {}, Ask {}",
                call.key.strike,
                call.bid
                    .as_ref()
                    .map_or_else(|| "N/A".to_string(), ToString::to_string),
                call.ask
                    .as_ref()
                    .map_or_else(|| "N/A".to_string(), ToString::to_string)
            );
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

    if let Some(mean) = price_target.mean.as_ref() {
        println!("Price Target: {mean}");
    }
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

Yahoo currently returns empty ESG responses for common symbols. `Ticker::sustainability()` mirrors
Python yfinance for that provider response by returning an empty summary instead of treating it as
a hard failure.

```rust
use std::fmt::Display;
use yfinance_rs::{Ticker, YfClient};

fn display_opt<T: Display>(value: Option<T>) -> String {
    value.map_or_else(|| "N/A".to_string(), |value| value.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, "AAPL");

    let summary = ticker.sustainability().await?;

    if let Some(scores) = summary.scores.as_ref() {
        println!("Environmental Score: {}", display_opt(scores.environmental));
        println!("Social Score: {}", display_opt(scores.social));
        println!("Governance Score: {}", display_opt(scores.governance));
    } else {
        println!("No ESG scores returned by Yahoo for this ticker.");
    }

    Ok(())
}
```

### Advanced Client Configuration

```rust
use yfinance_rs::{
    Ticker, YfClientBuilder,
    core::client::{Backoff, CacheEndpoint, CacheMode, RetryConfig},
};
use std::time::Duration;

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
        .cache_ttl(Duration::from_secs(600))
        .cache_ttl_for(CacheEndpoint::Quote, Duration::from_secs(5))
        .build()?;

    let ticker = Ticker::new(&client, "AAPL")
        .cache_mode(CacheMode::Use)
        .retry_policy(Some(RetryConfig {
            max_retries: 5,
            ..Default::default()
        }));
    
    let quote = ticker.quote().await?;
    if let Some(price) = quote.price.as_ref() {
        println!("Latest price for AAPL with custom client: {price}");
    }

    Ok(())
}

```

#### Custom Reqwest Client

For full control over HTTP configuration, you can provide your own reqwest client:
`YfClient` handles Yahoo auth cookies internally, so the custom client does not
need `cookie_store(true)` unless your own reqwest usage requires it.

```rust
use yfinance_rs::{CacheEndpoint, CacheMode, Ticker, YfClient};
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
        .cache_ttl_for(CacheEndpoint::Quote, Duration::from_secs(5))
        .build()?;

    let ticker = Ticker::new(&client, "AAPL").cache_mode(CacheMode::Use);
    let quote = ticker.quote().await?;
    if let Some(price) = quote.price.as_ref() {
        println!("Latest price for AAPL: {price}");
    }

    Ok(())
}
```

#### Proxy Configuration

You can configure general or scheme-specific proxies through fallible builder methods:

```rust
use yfinance_rs::{Ticker, YfClient};
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

    let ticker = Ticker::new(&client, "AAPL");
    let quote = ticker.quote().await?;
    if let Some(price) = quote.price.as_ref() {
        println!("Latest price for AAPL via proxy: {price}");
    }

    Ok(())
}
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Please see our [Contributing Guide](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md). We welcome pull requests and issues.

## Changelog

See **[CHANGELOG.md](https://github.com/gramistella/yfinance-rs/blob/main/CHANGELOG.md)** for release notes and breaking changes.
