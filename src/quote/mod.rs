use crate::core::client::CacheMode;
use crate::core::client::RetryConfig;
use crate::core::{Quote, YfClient, YfError, quotes as core_quotes};

/// A convenience function to fetch quotes for multiple symbols with default settings.
pub async fn quotes<I, S>(client: &YfClient, symbols: I) -> Result<Vec<Quote>, YfError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    QuotesBuilder::new(client.clone())
        .symbols(symbols)
        .fetch()
        .await
}

/// A builder for fetching quotes for one or more symbols.
pub struct QuotesBuilder {
    client: YfClient,
    symbols: Vec<String>,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl QuotesBuilder {
    /// Creates a new `QuotesBuilder`.
    pub fn new(client: YfClient) -> Self {
        Self {
            client,
            symbols: Vec::new(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for this specific API call.
    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call.
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Replaces the current list of symbols with a new list.
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a single symbol to the list.
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }

    /// Executes the request and fetches the quotes.
    pub async fn fetch(self) -> Result<Vec<crate::core::Quote>, crate::core::YfError> {
        if self.symbols.is_empty() {
            return Err(crate::core::YfError::Data(
                "quotes: at least one symbol required".into(),
            ));
        }

        let symbol_slices: Vec<&str> = self.symbols.iter().map(AsRef::as_ref).collect();
        let results = core_quotes::fetch_v7_quotes(
            &self.client,
            &symbol_slices,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await?;

        Ok(results.into_iter().map(map_v7_to_public).collect())
    }
}

fn map_v7_to_public(n: core_quotes::V7QuoteNode) -> Quote {
    Quote {
        symbol: n.symbol.unwrap_or_default(),
        regular_market_price: n.regular_market_price,
        regular_market_previous_close: n.regular_market_previous_close,
        currency: n.currency,
        exchange: n
            .full_exchange_name
            .or(n.exchange)
            .or(n.market)
            .or(n.market_cap_figure_exchange),
        market_state: n.market_state,
    }
}
