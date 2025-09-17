use chrono::{Duration, Utc};
use std::time::Duration as StdDuration;
use yfinance_rs::core::conversions::money_to_f64;
use yfinance_rs::{
    DownloadBuilder, Interval, NewsTab, StreamBuilder, StreamMethod, Ticker, YfClient,
    YfClientBuilder, YfError,
};

#[tokio::main]
async fn main() -> Result<(), YfError> {
    let client = YfClientBuilder::default()
        .timeout(StdDuration::from_secs(5))
        .build()?;

    section_info(&client).await?;
    section_fast_info(&client).await?;
    section_batch_quotes(&client).await?;
    section_download(&client).await?;
    section_options(&client).await?;
    section_stream(&client).await?;
    section_news(&client).await?;
    Ok(())
}

async fn section_info(client: &YfClient) -> Result<(), YfError> {
    let msft = Ticker::new(client, "MSFT");
    let info = msft.info().await?;
    println!("--- Ticker Info for {} ---", info.symbol);
    println!("Name: {}", info.short_name.unwrap_or_default());
    println!("Industry: {}", info.industry.unwrap_or_default());
    println!("Website: {}", info.website.unwrap_or_default());
    println!(
        "Mean Analyst Target: ${:.2}",
        info.target_mean_price.unwrap_or_default()
    );
    println!("ESG Score: {:.2}", info.total_esg_score.unwrap_or_default());
    println!();
    Ok(())
}

async fn section_fast_info(client: &YfClient) -> Result<(), YfError> {
    println!("--- Fast Info for NVDA ---");
    let nvda = Ticker::new(client, "NVDA");
    let fast_info = nvda.fast_info().await?;
    println!(
        "{} is trading at ${:.2} in {}",
        fast_info.symbol,
        fast_info.last_price,
        fast_info.exchange.unwrap_or_default()
    );
    println!();
    Ok(())
}

async fn section_batch_quotes(client: &YfClient) -> Result<(), YfError> {
    println!("--- Batch Quotes for Multiple Symbols ---");
    let quotes = yfinance_rs::quotes(client, vec!["AMD", "INTC", "QCOM"]).await?;
    for quote in quotes {
        println!(
            "  {}: ${:.2}",
            quote.symbol,
            quote.price.as_ref().map(money_to_f64).unwrap_or_default()
        );
    }
    println!();
    Ok(())
}

async fn section_download(client: &YfClient) -> Result<(), YfError> {
    let symbols = vec!["AAPL", "GOOG", "TSLA"];
    let today = Utc::now();
    let three_months_ago = today - Duration::days(90);
    println!("--- Historical Data for Multiple Symbols ---");
    let results = DownloadBuilder::new(client)
        .symbols(symbols)
        .between(three_months_ago, today)
        .interval(Interval::D1)
        .run()
        .await?;
    for (symbol, candles) in &results.series {
        println!("{} has {} data points.", symbol, candles.len());
        if let Some(last_candle) = candles.last() {
            println!(
                "  Last close price: ${:.2}",
                money_to_f64(&last_candle.close)
            );
        }
    }
    println!();
    Ok(())
}

async fn section_options(client: &YfClient) -> Result<(), YfError> {
    let aapl = Ticker::new(client, "AAPL");
    let expirations = aapl.options().await?;
    if let Some(first_expiry) = expirations.first() {
        println!("--- Options Chain for AAPL ({first_expiry}) ---");
        let chain = aapl.option_chain(Some(*first_expiry)).await?;
        println!(
            "  Found {} calls and {} puts.",
            chain.calls.len(),
            chain.puts.len()
        );
        if let Some(first_call) = chain.calls.first() {
            println!(
                "  First call option: {} @ ${:.2}",
                first_call.contract_symbol,
                money_to_f64(&first_call.strike)
            );
        }
    }
    println!();
    Ok(())
}

async fn section_stream(client: &YfClient) -> Result<(), YfError> {
    println!("--- Streaming Real-time Quotes for MSFT and GOOG ---");
    println!("(Streaming for 10 seconds or until stopped...)");
    let (handle, mut receiver) = StreamBuilder::new(client)
        .symbols(vec!["MSFT", "GOOG"])
        .method(StreamMethod::WebsocketWithFallback)
        .start()?;

    let stream_task = tokio::spawn(async move {
        let mut count = 0;
        while let Some(update) = receiver.recv().await {
            println!(
                "[{}] {} @ {:.2} (ts={})",
                update.ts,
                update.symbol,
                update.last_price.unwrap_or_default(),
                update.ts
            );
            count += 1;
            if count >= 10 {
                break;
            }
        }
        println!("Finished streaming after {count} updates.");
    });

    tokio::select! {
        () = tokio::time::sleep(StdDuration::from_secs(10)) => {
            println!("Stopping stream due to timeout.");
            handle.stop().await;
        }
        _ = stream_task => {
            println!("Stream task completed on its own.");
        }
    };
    Ok(())
}

async fn section_news(client: &YfClient) -> Result<(), YfError> {
    let tesla_news = Ticker::new(client, "TSLA");
    let articles = tesla_news
        .news_builder()
        .tab(NewsTab::PressReleases)
        .count(5)
        .fetch()
        .await?;
    println!("\n--- Latest 5 Press Releases for TSLA ---");
    for article in articles {
        println!(
            "- {} by {}",
            article.title,
            article.publisher.unwrap_or_else(|| "Unknown".to_string())
        );
    }
    Ok(())
}
