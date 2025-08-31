use crate::{
    Address, Profile, YfClient, YfError, analysis,
    core::client::{CacheMode, RetryConfig},
    esg,
};
use serde::Serialize;

/// A comprehensive summary of a ticker's data, aggregated from multiple API endpoints.
///
/// This struct is the result of `Ticker::info()` and contains a wide range of data including
/// quote details, company/fund profile, analyst ratings, and ESG scores. Fields will be `None`
/// if the corresponding data is not available for the ticker or if a non-essential API call fails.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Info {
    // --- Quote Data ---
    /// The ticker symbol of the security.
    pub symbol: String,
    /// The short name of the security.
    pub short_name: Option<String>,
    /// The last traded price in the regular market session.
    pub regular_market_price: Option<f64>,
    /// The closing price of the previous regular market session.
    pub regular_market_previous_close: Option<f64>,
    /// The currency in which the security is traded.
    pub currency: Option<String>,
    /// The full name of the exchange where the security is traded.
    pub exchange: Option<String>,
    /// The current state of the market for this security (e.g., "REGULAR", "PRE", "POST").
    pub market_state: Option<String>,

    // --- Profile Data ---
    // Company-specific
    /// The business sector the company operates in.
    pub sector: Option<String>,
    /// The specific industry within the sector.
    pub industry: Option<String>,
    /// The company's official website.
    pub website: Option<String>,
    /// A summary of the company's business operations.
    pub summary: Option<String>,
    /// The physical address of the company's headquarters.
    pub address: Option<Address>,
    // Fund-specific
    /// The family of funds it belongs to (e.g., "iShares").
    pub family: Option<String>,
    /// The legal type of the fund (e.g., "Exchange Traded Fund").
    pub fund_kind: Option<String>,
    // Common
    /// The International Securities Identification Number.
    pub isin: Option<String>,

    // --- Analysis Data ---
    /// The mean analyst price target.
    pub target_mean_price: Option<f64>,
    /// The highest analyst price target.
    pub target_high_price: Option<f64>,
    /// The lowest analyst price target.
    pub target_low_price: Option<f64>,
    /// The number of analysts providing an opinion.
    pub number_of_analyst_opinions: Option<u32>,
    /// The mean recommendation score (e.g., 1.0 for Strong Buy, 5.0 for Strong Sell).
    pub recommendation_mean: Option<f64>,
    /// The categorical key for the mean recommendation (e.g., "buy", "hold").
    pub recommendation_key: Option<String>,

    // --- ESG Data ---
    /// The total ESG score, a weighted average of the three component scores.
    pub total_esg_score: Option<f64>,
    /// The environmental score, measuring the company's impact on the environment.
    pub environment_score: Option<f64>,
    /// The social score, measuring performance on social issues.
    pub social_score: Option<f64>,
    /// The governance score, measuring corporate governance practices.
    pub governance_score: Option<f64>,
}

/// Private helper to handle optional async results, logging errors in debug mode.
fn log_err_async<T>(res: Result<T, YfError>, name: &str, symbol: &str) -> Option<T> {
    match res {
        Ok(data) => Some(data),
        Err(e) => {
            if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                eprintln!("YF_DEBUG(info): failed to fetch '{name}' for {symbol}: {e}");
            }
            None
        }
    }
}

pub(super) async fn fetch_info(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Info, YfError> {
    // Run all fetches concurrently
    let (quote_res, profile_res, price_target_res, rec_summary_res, esg_res) = tokio::join!(
        crate::ticker::quote::fetch_quote(client, symbol, cache_mode, retry_override),
        Profile::load(client, symbol),
        analysis::AnalysisBuilder::new(client, symbol)
            .cache_mode(cache_mode)
            .retry_policy(retry_override.cloned())
            .analyst_price_target(),
        analysis::AnalysisBuilder::new(client, symbol)
            .cache_mode(cache_mode)
            .retry_policy(retry_override.cloned())
            .recommendations_summary(),
        esg::EsgBuilder::new(client, symbol)
            .cache_mode(cache_mode)
            .retry_policy(retry_override.cloned())
            .fetch()
    );

    // Profile is essential. If it fails, we can't determine company vs. fund.
    let profile = profile_res?;

    // Use the generic helper for each fallible fetch.
    let quote = log_err_async(quote_res, "quote", symbol);
    let price_target = log_err_async(price_target_res, "price target", symbol);
    let rec_summary = log_err_async(rec_summary_res, "recommendation summary", symbol);
    let esg_scores = log_err_async(esg_res, "esg scores", symbol);

    // Extract profile-specific data using a more idiomatic match expression.
    let (sector, industry, website, summary, address, isin, family, fund_kind) = match profile {
        Profile::Company(c) => (
            c.sector, c.industry, c.website, c.summary, c.address, c.isin,
            None, // No family for a company
            None, // No fund_kind for a company
        ),
        Profile::Fund(f) => (
            None, // No sector for a fund
            None, // No industry for a fund
            None, // No website for a fund
            None, // No summary for a fund
            None, // No address for a fund
            f.isin,
            f.family,
            Some(f.kind),
        ),
    };

    let info = Info {
        // From Quote or default to symbol
        symbol: quote
            .as_ref()
            .map_or_else(|| symbol.to_string(), |q| q.symbol.clone()),
        short_name: quote.as_ref().and_then(|q| q.shortname.clone()),
        regular_market_price: quote.as_ref().and_then(|q| q.regular_market_price),
        regular_market_previous_close: quote.as_ref().and_then(|q| q.regular_market_previous_close),
        currency: quote.as_ref().and_then(|q| q.currency.clone()),
        exchange: quote.as_ref().and_then(|q| q.exchange.clone()),
        market_state: quote.as_ref().and_then(|q| q.market_state.clone()),

        // From Profile
        sector,
        industry,
        website,
        summary,
        address,
        isin,
        family,
        fund_kind,

        // From Analysis
        target_mean_price: price_target.as_ref().and_then(|pt| pt.mean),
        target_high_price: price_target.as_ref().and_then(|pt| pt.high),
        target_low_price: price_target.as_ref().and_then(|pt| pt.low),
        number_of_analyst_opinions: price_target.as_ref().and_then(|pt| pt.number_of_analysts),
        recommendation_mean: rec_summary.as_ref().and_then(|rs| rs.mean),
        recommendation_key: rec_summary.as_ref().and_then(|rs| rs.mean_key.clone()),

        // From ESG
        total_esg_score: esg_scores.as_ref().and_then(|esg| esg.total_esg),
        environment_score: esg_scores.as_ref().and_then(|esg| esg.environment_score),
        social_score: esg_scores.as_ref().and_then(|esg| esg.social_score),
        governance_score: esg_scores.as_ref().and_then(|esg| esg.governance_score),
    };

    Ok(info)
}
