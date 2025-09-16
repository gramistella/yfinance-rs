use chrono::Duration;
use yfinance_rs::{SearchBuilder, Ticker, YfClientBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClientBuilder::default()
        .timeout(Duration::seconds(5).to_std()?)
        .build()?;

    // --- Part 1: Fetching ESG Scores ---
    let msft_ticker = Ticker::new(&client, "MSFT");
    let esg_scores = msft_ticker.sustainability().await;

    println!("--- ESG Scores for MSFT ---");
    match esg_scores {
        Ok(scores) => {
            let total_esg = [scores.environmental, scores.social, scores.governance]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let total_esg_score = if total_esg.is_empty() {
                0.0
            } else {
                total_esg.iter().sum::<f64>() / (total_esg.len() as f64)
            };
            println!("Total ESG Score: {:.2}", total_esg_score);
            println!(
                "Environmental Score: {:.2}",
                scores.environmental.unwrap_or_default()
            );
            println!("Social Score: {:.2}", scores.social.unwrap_or_default());
            println!(
                "Governance Score: {:.2}",
                scores.governance.unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("Failed to fetch ESG scores: {}", e);
        }
    }
    println!("--------------------------------------\n");

    // --- Part 2: Fetching Analyst Ratings ---
    let tsla_ticker = Ticker::new(&client, "TSLA");
    let recommendations = tsla_ticker.recommendations().await;

    println!("--- Analyst Recommendations for TSLA ---");
    match recommendations {
        Ok(recs) => {
            if let Some(latest) = recs.first() {
                println!(
                    "Latest Recommendation Period ({}): Strong Buy: {:?}, Buy: {:?}, Hold: {:?}, Sell: {:?}, Strong Sell: {:?}",
                    latest.period,
                    latest.strong_buy,
                    latest.buy,
                    latest.hold,
                    latest.sell,
                    latest.strong_sell
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch recommendations: {}", e);
        }
    }

    let upgrades = tsla_ticker.upgrades_downgrades().await;
    if let Ok(upgrades_list) = upgrades {
        println!("\nRecent Upgrades/Downgrades:");
        for upgrade in upgrades_list.iter().take(3) {
            println!(
                "  - Firm: {} | Action: {} | From: {} | To: {}",
                upgrade.firm.as_deref().unwrap_or("N/A"),
                upgrade
                    .action
                    .as_ref()
                    .map(|a| a.to_string())
                    .unwrap_or("N/A".to_string()),
                upgrade
                    .from_grade
                    .as_ref()
                    .map(|g| g.to_string())
                    .unwrap_or("N/A".to_string()),
                upgrade
                    .to_grade
                    .as_ref()
                    .map(|g| g.to_string())
                    .unwrap_or("N/A".to_string())
            );
        }
    }
    println!("--------------------------------------\n");

    // --- Part 3: Using the Search API ---
    let query = "Apple Inc.";
    let search_results = SearchBuilder::new(&client, query).fetch().await;

    println!("--- Searching for '{}' ---", query);
    match search_results {
        Ok(results) => {
            println!("Found {} results:", results.quotes.len());
            for quote in results.quotes.iter().take(5) {
                println!(
                    "  - {} ({}): {}",
                    quote.symbol,
                    quote.quote_type.as_deref().unwrap_or("N/A"),
                    quote.longname.as_deref().unwrap_or("N/A")
                );
            }
        }
        Err(e) => {
            eprintln!("Search failed: {}", e);
        }
    }
    println!("--------------------------------------");

    Ok(())
}
