use crate::{
    YfClient, YfError, analysis,
    core::client::{CacheMode, RetryConfig},
    core::conversions::i64_to_datetime,
    esg,
    profile::Profile,
};
use paft::aggregates::Info;

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
    let (quote, profile) =
        Box::pin(fetch_info_parts(client, symbol, cache_mode, retry_override)).await?;
    let isin = extract_isin(&profile);
    Ok(assemble_info(symbol, quote.as_ref(), isin))
}

async fn fetch_info_parts(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<(Option<crate::Quote>, Profile), YfError> {
    let (quote_res, profile_res, _price_target_res, _rec_summary_res, _esg_res) = tokio::join!(
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
    Ok((quote, profile))
}

fn extract_isin(profile: &Profile) -> Option<paft::domain::Isin> {
    match profile {
        Profile::Company(c) => c.isin.clone(),
        Profile::Fund(f) => f.isin.clone(),
    }
}

fn assemble_info(
    symbol: &str,
    quote: Option<&crate::Quote>,
    isin: Option<paft::domain::Isin>,
) -> Info {
    Info {
        symbol: quote.map_or_else(
            || paft::domain::Symbol::new(symbol).expect("invalid symbol"),
            |q| q.symbol.clone(),
        ),
        name: quote.and_then(|q| q.shortname.clone()),
        isin,
        exchange: quote.and_then(|q| q.exchange.clone()),
        market_state: quote.and_then(|q| q.market_state),
        currency: quote.and_then(|q| {
            q.price
                .as_ref()
                .map(|m| m.currency().clone())
                .or_else(|| q.previous_close.as_ref().map(|m| m.currency().clone()))
        }),
        last: quote.and_then(|q| q.price.clone()),
        open: None,
        high: None,
        low: None,
        previous_close: quote.and_then(|q| q.previous_close.clone()),
        day_range_low: None,
        day_range_high: None,
        fifty_two_week_low: None,
        fifty_two_week_high: None,
        volume: None,
        average_volume: None,
        market_cap: None,
        shares_outstanding: None,
        eps_ttm: None,
        pe_ttm: None,
        dividend_yield: None,
        ex_dividend_date: None,
        as_of: Some(i64_to_datetime(chrono::Utc::now().timestamp())),
    }
}
