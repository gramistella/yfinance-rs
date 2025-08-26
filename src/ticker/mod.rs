mod model;
mod options;
mod quote;

pub use model::{FastInfo, OptionChain, OptionContract};

use crate::{
    Quote, YfClient, YfError,
    analysis::AnalysisBuilder,
    core::client::{CacheMode, RetryConfig},
    fundamentals::FundamentalsBuilder,
    history::HistoryBuilder,
};

/// A high-level interface for a single ticker symbol, providing convenient access to all available data.
///
/// This struct is designed to be the primary entry point for users who want to fetch
/// various types of financial data for a specific security, similar to the `Ticker`
/// object in the Python `yfinance` library.
///
/// A `Ticker` is created with a [`YfClient`] and a symbol. It then provides methods
/// to fetch quotes, historical prices, options chains, financials, and more.
///
/// # Example
///
/// ```no_run
/// # use yfinance_rs::{Ticker, YfClient};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = YfClient::default();
/// let ticker = Ticker::new(client, "TSLA");
///
/// // Get the latest quote
/// let quote = ticker.quote().await?;
/// println!("Tesla's last price: {}", quote.regular_market_price.unwrap_or_default());
///
/// // Get historical prices for the last year
/// let history = ticker.history(Some(yfinance_rs::Range::Y1), None, false).await?;
/// println!("Fetched {} days of history.", history.len());
/// # Ok(())
/// # }
/// ```
pub struct Ticker {
    #[doc(hidden)]
    pub(crate) client: YfClient,
    #[doc(hidden)]
    pub(crate) symbol: String,
    #[doc(hidden)]
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl Ticker {
    /// Creates a new `Ticker` for a given symbol.
    ///
    /// This is the standard way to create a ticker instance with default API endpoints.
    pub fn new(client: YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client,
            symbol: symbol.into(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for all subsequent API calls made by this `Ticker` instance.
    ///
    /// This allows you to override the client's default cache behavior for a specific ticker.
    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the client's default retry policy for all subsequent API calls made by this `Ticker` instance.
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Returns a `HistoryBuilder` to construct a detailed query for historical price data.
    pub fn history_builder(&self) -> HistoryBuilder<'_> {
        HistoryBuilder::new(&self.client, &self.symbol)
    }

    /* ---------------- Quotes ---------------- */

    /// Fetches a detailed quote for the ticker.
    pub async fn quote(&self) -> Result<Quote, YfError> {
        quote::fetch_quote(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches a "fast" info quote, containing the most essential price and market data.
    pub async fn fast_info(&self) -> Result<FastInfo, YfError> {
        let q = self.quote().await?;
        let last = q
            .regular_market_price
            .or(q.regular_market_previous_close)
            .ok_or_else(|| YfError::Data("quote missing last/previous price".into()))?;

        Ok(FastInfo {
            symbol: q.symbol,
            last_price: last,
            previous_close: q.regular_market_previous_close,
            currency: q.currency,
            exchange: q.exchange,
            market_state: q.market_state,
        })
    }

    /* ---------------- History helpers ---------------- */

    /// Fetches historical price candles with default settings.
    ///
    /// Prices are automatically adjusted for splits and dividends. For more control, use [`history_builder`].
    ///
    /// # Arguments
    /// * `range` - The relative time range for the data (e.g., `1y`, `6mo`). Defaults to `6mo` if `None`.
    /// * `interval` - The time interval for each candle (e.g., `1d`, `1wk`). Defaults to `1d` if `None`.
    /// * `prepost` - Whether to include pre-market and post-market data for intraday intervals.
    pub async fn history(
        &self,
        range: Option<crate::Range>,
        interval: Option<crate::Interval>,
        prepost: bool,
    ) -> Result<Vec<crate::Candle>, crate::core::YfError> {
        let mut hb = self.history_builder();
        if let Some(r) = range {
            hb = hb.range(r);
        }
        if let Some(i) = interval {
            hb = hb.interval(i);
        }
        hb = hb
            .auto_adjust(true)
            .prepost(prepost)
            .actions(true)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone());
        hb.fetch().await
    }

    /// Fetches all corporate actions (dividends and splits) for the given range.
    ///
    /// Defaults to the maximum available range if `None`.
    pub async fn actions(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<crate::Action>, YfError> {
        let mut hb = self.history_builder();
        hb = hb.range(range.unwrap_or(crate::Range::Max));
        let resp = hb.auto_adjust(true).actions(true).fetch_full().await?;
        Ok(resp.actions)
    }

    /// Fetches all dividend payments for the given range.
    ///
    /// Returns a `Vec` of tuples containing `(timestamp, amount)`.
    /// Defaults to the maximum available range if `None`.
    pub async fn dividends(&self, range: Option<crate::Range>) -> Result<Vec<(i64, f64)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Dividend { ts, amount } => Some((ts, amount)),
                _ => None,
            })
            .collect())
    }

    /// Fetches all stock splits for the given range.
    ///
    /// Returns a `Vec` of tuples containing `(timestamp, numerator, denominator)`.
    /// Defaults to the maximum available range if `None`.
    pub async fn splits(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<(i64, u32, u32)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Split {
                    ts,
                    numerator,
                    denominator,
                } => Some((ts, numerator, denominator)),
                _ => None,
            })
            .collect())
    }

    /// Fetches the metadata associated with the ticker's historical data, such as timezone.
    pub async fn get_history_metadata(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Option<crate::HistoryMeta>, crate::core::YfError> {
        let mut hb = self
            .history_builder()
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone());
        if let Some(r) = range {
            hb = hb.range(r);
        }
        let resp = hb.fetch_full().await?;
        Ok(resp.meta)
    }

    /* ---------------- Options ---------------- */

    /// Fetches the available expiration dates for the ticker's options as Unix timestamps.
    pub async fn options(&self) -> Result<Vec<i64>, YfError> {
        options::expiration_dates(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Fetches the full option chain (calls and puts) for a specific expiration date.
    ///
    /// If `date` is `None`, fetches the chain for the nearest expiration date.
    pub async fn option_chain(&self, date: Option<i64>) -> Result<OptionChain, YfError> {
        options::option_chain(
            &self.client,
            &self.symbol,
            date,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /* ---------------- Analysis convenience ---------------- */

    fn analysis_builder(&self) -> AnalysisBuilder {
        AnalysisBuilder::new(self.client.clone(), &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the analyst recommendation trend.
    pub async fn recommendations(&self) -> Result<Vec<crate::RecommendationRow>, YfError> {
        self.analysis_builder().recommendations().await
    }

    /// Fetches a summary of the latest analyst recommendations.
    pub async fn recommendations_summary(&self) -> Result<crate::RecommendationSummary, YfError> {
        self.analysis_builder().recommendations_summary().await
    }

    /// Fetches the history of analyst upgrades and downgrades.
    pub async fn upgrades_downgrades(&self) -> Result<Vec<crate::UpgradeDowngradeRow>, YfError> {
        self.analysis_builder().upgrades_downgrades().await
    }

    /// Fetches the analyst price target.
    pub async fn analyst_price_target(&self) -> Result<crate::PriceTarget, YfError> {
        self.analysis_builder().analyst_price_target().await
    }

    /* ---------------- Fundamentals convenience ---------------- */

    fn fundamentals_builder(&self) -> FundamentalsBuilder {
        FundamentalsBuilder::new(self.client.clone(), &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the annual income statement.
    pub async fn income_stmt(&self) -> Result<Vec<crate::IncomeStatementRow>, YfError> {
        self.fundamentals_builder().income_statement(false).await
    }

    /// Fetches the quarterly income statement.
    pub async fn quarterly_income_stmt(&self) -> Result<Vec<crate::IncomeStatementRow>, YfError> {
        self.fundamentals_builder().income_statement(true).await
    }

    /// Fetches the annual balance sheet.
    pub async fn balance_sheet(&self) -> Result<Vec<crate::BalanceSheetRow>, YfError> {
        self.fundamentals_builder().balance_sheet(false).await
    }

    /// Fetches the quarterly balance sheet.
    pub async fn quarterly_balance_sheet(&self) -> Result<Vec<crate::BalanceSheetRow>, YfError> {
        self.fundamentals_builder().balance_sheet(true).await
    }

    /// Fetches the annual cash flow statement.
    pub async fn cashflow(&self) -> Result<Vec<crate::CashflowRow>, YfError> {
        self.fundamentals_builder().cashflow(false).await
    }

    /// Fetches the quarterly cash flow statement.
    pub async fn quarterly_cashflow(&self) -> Result<Vec<crate::CashflowRow>, YfError> {
        self.fundamentals_builder().cashflow(true).await
    }

    /// Fetches earnings history and estimates.
    pub async fn earnings(&self) -> Result<crate::Earnings, YfError> {
        self.fundamentals_builder().earnings().await
    }

    /// Fetches corporate calendar events like earnings dates.
    pub async fn calendar(&self) -> Result<crate::FundCalendar, YfError> {
        self.fundamentals_builder().calendar().await
    }
}
