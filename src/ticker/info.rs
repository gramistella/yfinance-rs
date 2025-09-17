use crate::{
    YfClient, YfError, analysis,
    core::client::{CacheMode, RetryConfig},
    core::conversions::*,
    esg,
    profile::Profile,
    ticker::model::Info,
};

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
        crate::profile::load_profile(client, symbol),
        analysis::AnalysisBuilder::new(client, symbol)
            .cache_mode(cache_mode)
            .retry_policy(retry_override.cloned())
            .analyst_price_target(None),
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
        regular_market_price: quote
            .as_ref()
            .and_then(|q| q.price.as_ref().map(money_to_f64)),
        regular_market_previous_close: quote
            .as_ref()
            .and_then(|q| q.previous_close.as_ref().map(money_to_f64)),
        currency: quote.as_ref().and_then(|q| {
            q.price
                .as_ref()
                .and_then(money_to_currency_str)
                .or_else(|| q.previous_close.as_ref().and_then(money_to_currency_str))
        }),
        exchange: quote
            .as_ref()
            .and_then(|q| exchange_to_string(q.exchange.clone())),
        market_state: quote
            .as_ref()
            .and_then(|q| market_state_to_string(q.market_state.clone())),

        // From Profile
        sector,
        industry,
        website,
        summary,
        address,
        isin,
        family,
        fund_kind: fund_kind_to_string(fund_kind),

        // From Analysis
        target_mean_price: price_target
            .as_ref()
            .and_then(|pt| pt.mean.as_ref().map(money_to_f64)),
        target_high_price: price_target
            .as_ref()
            .and_then(|pt| pt.high.as_ref().map(money_to_f64)),
        target_low_price: price_target
            .as_ref()
            .and_then(|pt| pt.low.as_ref().map(money_to_f64)),
        number_of_analyst_opinions: price_target.as_ref().and_then(|pt| pt.number_of_analysts),
        recommendation_mean: rec_summary.as_ref().and_then(|rs| rs.mean),
        recommendation_key: None, // paft RecommendationSummary doesn't have mean_key field

        // From ESG (paft::fundamentals::EsgScores)
        total_esg_score: esg_scores.as_ref().and_then(|esg| {
            let mut sum = 0.0;
            let mut count = 0u32;
            if let Some(v) = esg.environmental {
                sum += v;
                count += 1;
            }
            if let Some(v) = esg.social {
                sum += v;
                count += 1;
            }
            if let Some(v) = esg.governance {
                sum += v;
                count += 1;
            }
            if count > 0 {
                Some(sum / f64::from(count))
            } else {
                None
            }
        }),
        environment_score: esg_scores.as_ref().and_then(|esg| esg.environmental),
        social_score: esg_scores.as_ref().and_then(|esg| esg.social),
        governance_score: esg_scores.as_ref().and_then(|esg| esg.governance),
    };

    Ok(info)
}
