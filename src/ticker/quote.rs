use url::Url;

use crate::{
    YfClient, YfError,
    core::{
        client::{CacheMode, RetryConfig},
        quotes,
    },
};

use super::model::Quote;

pub(crate) async fn fetch_quote(
    client: &YfClient,
    base: &Url,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Quote, YfError> {
    let symbols = [symbol];
    let mut results =
        quotes::fetch_v7_quotes(client, base, &symbols, cache_mode, retry_override).await?;

    let result = results
        .pop()
        .ok_or_else(|| YfError::Data(format!("no quote result found for symbol {}", symbol)))?;

    Ok(Quote {
        symbol: result.symbol.unwrap_or_else(|| symbol.to_string()),
        regular_market_price: result.regular_market_price,
        regular_market_previous_close: result.regular_market_previous_close,
        currency: result.currency,
        exchange: result
            .full_exchange_name
            .or(result.exchange)
            .or(result.market)
            .or(result.market_cap_figure_exchange),
        market_state: result.market_state,
    })
}
