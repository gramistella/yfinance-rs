use crate::ticker::{PriceTarget, RecommendationSummary};
use crate::{
    YfClient, YfError, analysis,
    core::client::{CacheMode, RetryConfig},
    core::conversions::{
        exchange_to_string, fund_kind_to_string, market_state_to_string, money_to_currency_str,
        money_to_f64,
    },
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
    let (quote, profile, price_target, rec_summary, esg_scores) =
        Box::pin(fetch_info_parts(client, symbol, cache_mode, retry_override)).await?;
    let ProfileFields {
        sector,
        industry,
        website,
        summary,
        address,
        isin,
        family,
        fund_kind,
    } = extract_profile_fields(&profile);
    let info = assemble_info(
        symbol,
        quote.as_ref(),
        sector,
        industry,
        website,
        summary,
        address,
        isin,
        family,
        fund_kind,
        price_target.as_ref(),
        rec_summary.as_ref(),
        esg_scores.as_ref(),
    );
    Ok(info)
}

async fn fetch_info_parts(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<
    (
        Option<crate::Quote>,
        Profile,
        Option<PriceTarget>,
        Option<RecommendationSummary>,
        Option<paft::fundamentals::esg::EsgSummary>,
    ),
    YfError,
> {
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

    let profile = profile_res?;
    let quote = log_err_async(quote_res, "quote", symbol);
    let price_target = log_err_async(price_target_res, "price target", symbol);
    let rec_summary = log_err_async(rec_summary_res, "recommendation summary", symbol);
    let esg_scores = log_err_async(esg_res, "esg scores", symbol);
    Ok((quote, profile, price_target, rec_summary, esg_scores))
}

struct ProfileFields {
    sector: Option<String>,
    industry: Option<String>,
    website: Option<String>,
    summary: Option<String>,
    address: Option<paft::fundamentals::profile::Address>,
    isin: Option<String>,
    family: Option<String>,
    fund_kind: Option<paft::fundamentals::profile::FundKind>,
}

fn extract_profile_fields(profile: &Profile) -> ProfileFields {
    match profile {
        Profile::Company(c) => ProfileFields {
            sector: c.sector.clone(),
            industry: c.industry.clone(),
            website: c.website.clone(),
            summary: c.summary.clone(),
            address: c.address.clone(),
            isin: c.isin.clone(),
            family: None,
            fund_kind: None,
        },
        Profile::Fund(f) => ProfileFields {
            sector: None,
            industry: None,
            website: None,
            summary: None,
            address: None,
            isin: f.isin.clone(),
            family: f.family.clone(),
            fund_kind: Some(f.kind.clone()),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn assemble_info(
    symbol: &str,
    quote: Option<&crate::Quote>,
    sector: Option<String>,
    industry: Option<String>,
    website: Option<String>,
    summary: Option<String>,
    address: Option<paft::fundamentals::profile::Address>,
    isin: Option<String>,
    family: Option<String>,
    fund_kind: Option<paft::fundamentals::profile::FundKind>,
    price_target: Option<&PriceTarget>,
    rec_summary: Option<&RecommendationSummary>,
    esg_scores: Option<&paft::fundamentals::esg::EsgSummary>,
) -> Info {
    let currency = quote.and_then(|q| {
        q.price
            .as_ref()
            .and_then(money_to_currency_str)
            .or_else(|| q.previous_close.as_ref().and_then(money_to_currency_str))
    });

    let total_esg_score = esg_scores.and_then(|summary| {
        let esg = summary.scores.as_ref()?;
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
    });

    Info {
        symbol: quote.map_or_else(|| symbol.to_string(), |q| q.symbol.clone()),
        short_name: quote.and_then(|q| q.shortname.clone()),
        regular_market_price: quote.and_then(|q| q.price.as_ref().map(money_to_f64)),
        regular_market_previous_close: quote
            .and_then(|q| q.previous_close.as_ref().map(money_to_f64)),
        currency,
        exchange: quote.and_then(|q| exchange_to_string(q.exchange.clone())),
        market_state: quote.and_then(|q| market_state_to_string(q.market_state.clone())),

        sector,
        industry,
        website,
        summary,
        address,
        isin,
        family,
        fund_kind: fund_kind_to_string(fund_kind),

        target_mean_price: price_target.and_then(|pt| pt.mean.as_ref().map(money_to_f64)),
        target_high_price: price_target.and_then(|pt| pt.high.as_ref().map(money_to_f64)),
        target_low_price: price_target.and_then(|pt| pt.low.as_ref().map(money_to_f64)),
        number_of_analyst_opinions: price_target.and_then(|pt| pt.number_of_analysts),
        recommendation_mean: rec_summary.and_then(|rs| rs.mean),
        recommendation_key: None,

        total_esg_score,
        environment_score: esg_scores.and_then(|s| s.scores.as_ref().and_then(|x| x.environmental)),
        social_score: esg_scores.and_then(|s| s.scores.as_ref().and_then(|x| x.social)),
        governance_score: esg_scores.and_then(|s| s.scores.as_ref().and_then(|x| x.governance)),
    }
}
