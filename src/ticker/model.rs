use serde::{Deserialize, Serialize};
use paft::fundamentals::Address;

// Re-export types from paft
pub use paft::market::{OptionChain, OptionContract};

/// Fast info structure containing essential quote data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FastInfo {
    /// Ticker symbol.
    pub symbol: String,
    /// Last traded price.
    pub last_price: f64,
    /// Previous close price.
    pub previous_close: Option<f64>,
    /// ISO currency code of prices.
    pub currency: Option<String>,
    /// Primary exchange name.
    pub exchange: Option<String>,
    /// Market state as a string.
    pub market_state: Option<String>,
}

/// Comprehensive info structure containing quote, profile, analysis, and ESG data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Info {
    // From Quote
    /// Ticker symbol.
    pub symbol: String,
    /// Short display name.
    pub short_name: Option<String>,
    /// Current regular market price.
    pub regular_market_price: Option<f64>,
    /// Previous session's close price.
    pub regular_market_previous_close: Option<f64>,
    /// ISO currency code of prices.
    pub currency: Option<String>,
    /// Primary exchange name.
    pub exchange: Option<String>,
    /// Market state as a string.
    pub market_state: Option<String>,

    // From Profile
    /// Sector for companies.
    pub sector: Option<String>,
    /// Industry for companies.
    pub industry: Option<String>,
    /// Company or fund website.
    pub website: Option<String>,
    /// Business summary/description.
    pub summary: Option<String>,
    /// Mailing address.
    pub address: Option<Address>,
    /// International Securities Identification Number.
    pub isin: Option<String>,
    /// Fund family name for funds.
    pub family: Option<String>,
    /// Fund kind/category.
    pub fund_kind: Option<String>,

    // From Analysis
    /// Analyst target mean price.
    pub target_mean_price: Option<f64>,
    /// Analyst target high price.
    pub target_high_price: Option<f64>,
    /// Analyst target low price.
    pub target_low_price: Option<f64>,
    /// Number of analyst opinions.
    pub number_of_analyst_opinions: Option<u32>,
    /// Recommendation mean.
    pub recommendation_mean: Option<f64>,
    /// Recommendation key text.
    pub recommendation_key: Option<String>,

    // From ESG
    /// Total ESG score (computed average of available components).
    pub total_esg_score: Option<f64>,
    /// Environmental score.
    pub environment_score: Option<f64>,
    /// Social score.
    pub social_score: Option<f64>,
    /// Governance score.
    pub governance_score: Option<f64>,
}
