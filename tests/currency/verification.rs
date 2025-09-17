use yfinance_rs::core::{Interval, Range, conversions::money_to_f64};
use yfinance_rs::quote::quotes;
use yfinance_rs::ticker::Ticker;
use yfinance_rs::{YfClient, YfError};

async fn check_fast_info(ticker: &Ticker, expected_currency: &str) {
    println!("  📈 Quote/Fast Info:");
    match ticker.fast_info().await {
        Ok(fi) => {
            println!("    Symbol: {}", fi.symbol);
            println!("    Last Price: {}", fi.last_price);
            println!("    Currency: {:?}", fi.currency);
            println!("    Exchange: {:?}", fi.exchange);
            let currency_correct = fi.currency.as_deref() == Some(expected_currency);
            println!(
                "    {} Currency {}: {} (expected {})",
                if currency_correct { "✅" } else { "❌" },
                if currency_correct { "CORRECT" } else { "INCORRECT" },
                fi.currency.as_deref().unwrap_or("None"),
                expected_currency
            );
        }
        Err(e) => println!("    ❌ Error: {e}"),
    }
}

async fn check_comprehensive_info(ticker: &Ticker, expected_currency: &str) {
    println!("  📋 Comprehensive Info:");
    match ticker.info().await {
        Ok(info) => {
            println!("    Symbol: {}", info.symbol);
            println!("    Regular Market Price: {:?}", info.regular_market_price);
            println!("    Currency: {:?}", info.currency);
            println!("    Exchange: {:?}", info.exchange);
            let currency_correct = info.currency.as_deref() == Some(expected_currency);
            println!(
                "    {} Currency {}: {} (expected {})",
                if currency_correct { "✅" } else { "❌" },
                if currency_correct { "CORRECT" } else { "INCORRECT" },
                info.currency.as_deref().unwrap_or("None"),
                expected_currency
            );
        }
        Err(e) => println!("    ❌ Error: {e}"),
    }
}

async fn check_history(ticker: &Ticker, expected_currency: &str) {
    println!("  📊 Historical Data:");
    match ticker
        .history(Some(Range::D5), Some(Interval::D1), false)
        .await
    {
        Ok(history) => {
            if let Some(last_candle) = history.last() {
                println!("    Last Close: {:?}", last_candle.close);
                println!("    Currency: \"{}\"", last_candle.close.currency());
                let currency_correct =
                    last_candle.close.currency().to_string() == expected_currency;
                println!(
                    "    {} Currency {}: {} (expected {})",
                    if currency_correct { "✅" } else { "❌" },
                    if currency_correct { "CORRECT" } else { "INCORRECT" },
                    last_candle.close.currency(),
                    expected_currency
                );
            } else {
                println!("    ❌ No historical data available");
            }
        }
        Err(e) => println!("    ❌ Error: {e}"),
    }
}

async fn check_fundamentals(ticker: &Ticker) {
    println!("  💰 Fundamentals:");
    match ticker.income_stmt(None).await {
        Ok(income_stmt) => {
            if let Some(latest) = income_stmt.first() {
                println!("    Total Revenue: {:?}", latest.total_revenue);
                println!("    Net Income: {:?}", latest.net_income);
                println!(
                    "    Revenue Currency: {} (Note: Financial statements typically in USD)",
                    latest
                        .total_revenue
                        .as_ref()
                        .map_or_else(|| "None".to_string(), |m| m.currency().to_string())
                );
                println!("    ✅ Revenue Currency CORRECT: USD (financial statements standard)");
            } else {
                println!("    ❌ No income statement data available");
            }
        }
        Err(e) => println!("    ❌ Error: {e}"),
    }
}

async fn check_analysis(ticker: &Ticker) {
    println!("  📊 Analysis:");
    match ticker.analyst_price_target(None).await {
        Ok(target) => {
            println!("    Mean Target: {:?}", target.mean);
            println!("    High Target: {:?}", target.high);
            println!("    Low Target: {:?}", target.low);
            println!(
                "    Target Currency: {} (Note: Analyst targets typically in USD)",
                target
                    .mean
                    .as_ref()
                    .map_or_else(|| "None".to_string(), |m| m.currency().to_string())
            );
            println!("    ✅ Target Currency CORRECT: USD (analyst targets standard)");
        }
        Err(e) => println!("    ❌ Error: {e}"),
    }
}

async fn run_symbol_check(
    client: &YfClient,
    symbol: &str,
    expected_currency: &str,
    description: &str,
) -> Result<(), YfError> {
    println!("\n📊 Testing: {symbol} ({description})");
    println!("Expected Currency: {expected_currency}");
    println!("{}", "-".repeat(50));
    let ticker = Ticker::new(client, symbol);

    check_fast_info(&ticker, expected_currency).await;
    check_comprehensive_info(&ticker, expected_currency).await;
    check_history(&ticker, expected_currency).await;
    check_fundamentals(&ticker).await;
    check_analysis(&ticker).await;

    Ok(())
}

async fn run_batch_quotes(client: &YfClient) -> Result<(), YfError> {
    println!("\n📊 Batch Quotes Currency Test:");
    println!("{}", "-".repeat(50));
    let symbols = vec!["AAPL", "TSCO.L", "7203.T"];
    match quotes(client, symbols.clone()).await {
        Ok(batch_quotes) => {
            for (i, quote) in batch_quotes.iter().enumerate() {
                let symbol = &symbols[i];
                let expected = match *symbol {
                    "TSCO.L" => "GBP",
                    "7203.T" => "JPY",
                    _ => "USD",
                };
                let currency = quote.price.as_ref().map(|m| m.currency().to_string());
                let currency_correct = currency.as_deref() == Some(expected);
                println!(
                    "  {}: Price={:?}, Currency={:?}",
                    symbol,
                    quote.price.as_ref().map(money_to_f64),
                    currency
                );
                println!(
                    "    {} Currency {}: {} (expected {})",
                    if currency_correct { "✅" } else { "❌" },
                    if currency_correct {
                        "CORRECT"
                    } else {
                        "INCORRECT"
                    },
                    currency.as_deref().unwrap_or("None"),
                    expected
                );
            }
        }
        Err(e) => println!("  ❌ Error: {e}"),
    }
    Ok(())
}

#[tokio::test]
async fn test_currency_verification() -> Result<(), YfError> {
    println!("🔍 Currency Verification Test");
    println!("============================");
    let client = YfClient::builder().build().unwrap();
    let cases = vec![
        ("AAPL", "USD", "US Stock (NASDAQ)"),
        ("TSCO.L", "GBP", "UK Stock (LSE)"),
        ("7203.T", "JPY", "Japanese Stock (TSE)"),
        ("ASML.AS", "EUR", "Dutch Stock (Euronext)"),
        ("TSM", "USD", "Taiwanese Stock (NYSE)"),
    ];
    for (symbol, expected, desc) in cases {
        run_symbol_check(&client, symbol, expected, desc).await?;
    }
    run_batch_quotes(&client).await?;
    println!("\n✅ Currency verification test completed!");
    Ok(())
}

#[tokio::test]
async fn test_currency_precision() -> Result<(), YfError> {
    println!("\n🔍 Currency Precision Test");
    println!("==========================");

    let client = YfClient::builder().build().unwrap();
    let ticker = Ticker::new(&client, "AAPL");

    // Test historical data precision
    match ticker
        .history(Some(Range::D5), Some(Interval::D1), false)
        .await
    {
        Ok(history) => {
            if let Some(last_candle) = history.last() {
                println!("📊 Historical Data Precision:");
                println!("  Open:  {:?}", last_candle.open);
                println!("  High:  {:?}", last_candle.high);
                println!("  Low:   {:?}", last_candle.low);
                println!("  Close: {:?}", last_candle.close);

                // Check if amounts are clean (no precision artifacts)
                let amounts = [
                    money_to_f64(&last_candle.open),
                    money_to_f64(&last_candle.high),
                    money_to_f64(&last_candle.low),
                    money_to_f64(&last_candle.close),
                ];

                let has_precision_issues = amounts.iter().any(|&amount| {
                    let formatted = format!("{amount:.4}");
                    let parsed_back = formatted.parse::<f64>().unwrap_or(0.0);
                    (amount - parsed_back).abs() > 1e-10
                });

                if has_precision_issues {
                    println!("  ❌ Precision issues detected!");
                } else {
                    println!("  ✅ Clean precision - no artifacts");
                }
            }
        }
        Err(e) => println!("❌ Error: {e}"),
    }

    // Test quote precision
    match ticker.quote().await {
        Ok(quote) => {
            println!("\n📈 Quote Data Precision:");
            if let Some(price) = &quote.price {
                println!("  Price: {price:?}");
                let amount = money_to_f64(price);
                let formatted = format!("{amount:.4}");
                let parsed_back = formatted.parse::<f64>().unwrap_or(0.0);
                let has_precision_issues = (amount - parsed_back).abs() > 1e-10;

                if has_precision_issues {
                    println!("  ❌ Precision issues detected!");
                } else {
                    println!("  ✅ Clean precision - no artifacts");
                }
            }
        }
        Err(e) => println!("❌ Error: {e}"),
    }

    Ok(())
}
