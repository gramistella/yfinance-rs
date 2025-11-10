use chrono::Duration;
use yfinance_rs::{StreamBuilder, StreamMethod, YfClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let symbols = vec!["TSLA", "GOOG"];

    println!("--- Polling for Real-time Quotes every 5 seconds ---");
    println!("(Polling for 20 seconds or until stopped...)");

    // Create a StreamBuilder explicitly configured for polling.
    let (handle, mut receiver) = StreamBuilder::new(&client)
        .symbols(symbols)
        .method(StreamMethod::Polling)
        .interval(Duration::seconds(5).to_std().unwrap())
        .diff_only(false) // Get updates even if price hasn't changed
        .start()?;

    let stream_task = tokio::spawn(async move {
        let mut count = 0;
        while let Some(update) = receiver.recv().await {
            println!(
                "[{}] {} @ {:.2} {}",
                update.ts,
                update.instrument,
                update
                    .price
                    .as_ref()
                    .map(yfinance_rs::core::conversions::money_to_f64)
                    .unwrap_or_default(),
                update
                    .volume
                    .map(|v| format!("({v} delta)"))
                    .unwrap_or_default()
            );
            count += 1;
        }
        println!("Finished polling after {count} updates.");
    });

    // Stop the stream after 20 seconds, regardless of how many updates were received.
    tokio::select! {
        () = tokio::time::sleep(Duration::seconds(20).to_std()?) => {
            println!("Stopping polling due to timeout.");
            handle.stop().await;
        }
        _ = stream_task => {
            println!("Polling task completed on its own.");
        }
    };

    Ok(())
}
