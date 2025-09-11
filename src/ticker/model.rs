use serde::{Deserialize, Serialize};
use paft::fundamentals::Address;

// Re-export types from paft
pub use paft::market::{OptionChain, OptionContract};

/// Fast info structure containing essential quote data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FastInfo {
    pub symbol: String,
    pub last_price: f64,
    pub previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,
}

/// Comprehensive info structure containing quote, profile, analysis, and ESG data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Info {
    // From Quote
    pub symbol: String,
    pub short_name: Option<String>,
    pub regular_market_price: Option<f64>,
    pub regular_market_previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,

    // From Profile
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub website: Option<String>,
    pub summary: Option<String>,
    pub address: Option<Address>,
    pub isin: Option<String>,
    pub family: Option<String>,
    pub fund_kind: Option<String>,

    // From Analysis
    pub target_mean_price: Option<f64>,
    pub target_high_price: Option<f64>,
    pub target_low_price: Option<f64>,
    pub number_of_analyst_opinions: Option<u32>,
    pub recommendation_mean: Option<f64>,
    pub recommendation_key: Option<String>,

    // From ESG
    pub total_esg_score: Option<f64>,
    pub environment_score: Option<f64>,
    pub social_score: Option<f64>,
    pub governance_score: Option<f64>,
}
