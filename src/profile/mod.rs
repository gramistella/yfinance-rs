//! Public profile types + loading strategy (API first, then scrape).
//!
//! Internals are split into:
//! - `api`:    quoteSummary v10 API path
//! - `scrape`: HTML scrape + JSON extraction path
//! - `internal`: common utilities for both API and scrape
//! - `debug`:  optional debug dump helpers (only in debug builds or with `debug-dumps` feature)

mod api;
mod scrape;

#[cfg(feature = "debug-dumps")]
pub(crate) mod debug;

use crate::{YfClient, YfError};

mod model;
pub use model::{Address, Company, Fund, Profile};

/// Helper to contain the API->Scrape fallback logic.
async fn load_with_fallback(client: &YfClient, symbol: &str) -> Result<Profile, YfError> {
    client.ensure_credentials().await?;

    match api::load_from_quote_summary_api(client, symbol).await {
        Ok(p) => Ok(p),
        Err(e) => {
            if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                eprintln!("YF_DEBUG: API call failed ({e}), falling back to scrape.");
            }
            scrape::load_from_scrape(client, symbol).await
        }
    }
}

/// Loads the profile for a given symbol.
///
/// This function will try to load the profile from the quote summary API first,
/// and fall back to scraping the quote page if the API fails.
///
/// # Errors
///
/// Returns `YfError` if the network request fails, the response cannot be parsed,
/// or the data for the symbol is not available.
pub async fn load_profile(client: &YfClient, symbol: &str) -> Result<Profile, YfError> {
    #[cfg(not(feature = "test-mode"))]
    {
        load_with_fallback(client, symbol).await
    }

    #[cfg(feature = "test-mode")]
    {
        use crate::core::client::ApiPreference;
        match client.api_preference() {
            ApiPreference::ApiThenScrape => load_with_fallback(client, symbol).await,
            ApiPreference::ApiOnly => {
                client.ensure_credentials().await?;
                api::load_from_quote_summary_api(client, symbol).await
            }
            ApiPreference::ScrapeOnly => scrape::load_from_scrape(client, symbol).await,
        }
    }
}
