use crate::core::client::CacheMode;
use crate::core::client::RetryConfig;
use crate::core::{Quote, YfClient, YfError, quotes as core_quotes};

/// Fetches quotes for multiple symbols.
///
/// # Errors
///
/// Returns `YfError` if the network request fails, the response cannot be parsed,
/// or the data for the symbols is not available.
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
    #[must_use]
    pub const fn new(client: YfClient) -> Self {
        Self {
            client,
            symbols: Vec::new(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for this specific API call.
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call.
    #[must_use]
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Replaces the current list of symbols with a new list.
    #[must_use]
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a single symbol to the list.
    #[must_use]
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }

    /// Fetches the quotes for the configured symbols.
    ///
    /// # Errors
    ///
    /// Returns `YfError` if no symbols were provided, the network request fails,
    /// the response cannot be parsed, or data for the symbols is not available.
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

        Ok(results.into_iter().map(Into::into).collect())
    }
}
