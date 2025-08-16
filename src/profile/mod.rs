//! Public profile types + loading strategy (API first, then scrape).
//!
//! Internals are split into:
//! - `api`:    quoteSummary v10 API path
//! - `scrape`: HTML scrape + JSON extraction path
//! - `internal`: common utilities for both API and scrape
//! - `debug`:  optional debug dump helpers (only in debug builds or with `debug-dumps` feature)

mod api;
mod scrape;

#[cfg(any(debug_assertions, feature = "debug-dumps"))]
pub(crate) mod debug;

use crate::{YfClient, YfError};

mod model;
pub use model::{Address, Company, Fund, Profile};

impl Profile {
    /// Load a profile for `symbol`. In normal builds:
    /// 1) Ensure credentials (cookie + crumb)
    /// 2) Try API; if it errors, fall back to scraping the HTML page.
    ///
    /// In `test-mode`, the behavior follows `ApiPreference`.
    pub async fn load(client: &mut YfClient, symbol: &str) -> Result<Profile, YfError> {
        #[cfg(not(feature = "test-mode"))]
        {
            client.ensure_credentials().await?;

            match api::load_from_quote_summary_api(client, symbol).await {
                Ok(p) => return Ok(p),
                Err(e) => {
                    if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                        eprintln!("YF_DEBUG: API call failed ({e}), falling back to scrape.");
                    }
                }
            }

            scrape::load_from_scrape(client, symbol).await
        }

        #[cfg(feature = "test-mode")]
        {
            use crate::core::client::ApiPreference;
            match client.api_preference() {
                ApiPreference::ApiThenScrape => {
                    client.ensure_credentials().await?;
                    match api::load_from_quote_summary_api(client, symbol).await {
                        Ok(p) => return Ok(p),
                        Err(e) => {
                            if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                                eprintln!(
                                    "YF_DEBUG: API call failed ({e}), falling back to scrape."
                                );
                            }
                        }
                    }
                    scrape::load_from_scrape(client, symbol).await
                }
                ApiPreference::ApiOnly => {
                    client.ensure_credentials().await?;
                    api::load_from_quote_summary_api(client, symbol).await
                }
                ApiPreference::ScrapeOnly => scrape::load_from_scrape(client, symbol).await,
            }
        }
    }
}
