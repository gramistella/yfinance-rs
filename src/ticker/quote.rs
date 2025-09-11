use crate::{
    Quote, YfClient, YfError,
    core::{
        client::{CacheMode, RetryConfig},
        quotes,
        conversions::*,
    },
};

pub async fn fetch_quote(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Quote, YfError> {
    let symbols = [symbol];
    let mut results = quotes::fetch_v7_quotes(client, &symbols, cache_mode, retry_override).await?;

    let result = results.pop().ok_or_else(|| {
        YfError::MissingData(format!("no quote result found for symbol {symbol}"))
    })?;

    Ok(Quote {
        symbol: result.symbol.unwrap_or_else(|| symbol.to_string()),
        shortname: result.short_name,
        price: result.regular_market_price.map(f64_to_money),
        previous_close: result.regular_market_previous_close.map(f64_to_money),
        exchange: string_to_exchange(result
            .full_exchange_name
            .or(result.exchange)
            .or(result.market)
            .or(result.market_cap_figure_exchange)),
        market_state: string_to_market_state(result.market_state),
    })
}
