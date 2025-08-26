use chrono::Duration;
use yfinance_rs::{SearchBuilder, Ticker, core::client::YfClientBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YfClientBuilder::default()
        .timeout(Duration::seconds(5).to_std()?)
        .build()?;

    // --- Part 1: Fetching ESG Scores ---
    let msft_ticker = Ticker::new(client.clone(), "MSFT");
    let esg_scores = msft_ticker.sustainability().await;

    println!("--- ESG Scores for MSFT ---");
    match esg_scores {
        Ok(scores) => {
            println!(
                "Total ESG Score: {:.2}",
                scores.total_esg.unwrap_or_default()
            );
            println!(
                "Environmental Score: {:.2}",
                scores.environment_score.unwrap_or_default()
            );
            println!(
                "Social Score: {:.2}",
                scores.social_score.unwrap_or_default()
            );
            println!(
                "Governance Score: {:.2}",
                scores.governance_score.unwrap_or_default()
            );
            println!(
                "Has controversial weapons involvement: {}",
                scores.involvement.controversial_weapons
            );
        }
        Err(e) => {
            eprintln!("Failed to fetch ESG scores: {}", e);
        }
    }
    println!("--------------------------------------\n");

    // --- Part 2: Fetching Analyst Ratings ---
    let tsla_ticker = Ticker::new(client.clone(), "TSLA");
    let recommendations = tsla_ticker.recommendations().await;

    println!("--- Analyst Recommendations for TSLA ---");
    match recommendations {
        Ok(recs) => {
            if let Some(latest) = recs.first() {
                println!(
                    "Latest Recommendation Period ({}): Strong Buy: {}, Buy: {}, Hold: {}, Sell: {}, Strong Sell: {}",
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
                upgrade.action.as_deref().unwrap_or("N/A"),
                upgrade.from_grade.as_deref().unwrap_or("N/A"),
                upgrade.to_grade.as_deref().unwrap_or("N/A")
            );
        }
    }
    println!("--------------------------------------\n");

    // --- Part 3: Using the Search API ---
    let client_for_search = client.clone();
    let query = "Apple Inc.";
    let search_results = SearchBuilder::new(client_for_search, query)?.fetch().await;

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
