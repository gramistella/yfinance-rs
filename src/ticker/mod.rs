mod components;
mod info;
mod isin;
mod model;
mod options;
mod quote;

pub use crate::core::models::{FastInfo, MovingAverages};
pub use model::{Info, OptionChain, OptionContract};

use crate::core::{Action, Candle, HistoryMeta, Interval, Quote, Range};
use crate::fundamentals::{Calendar, ShareCount};
use crate::holders::{
    InsiderRosterHolder, InsiderTransaction, InstitutionalHolder, MajorHolder,
    NetSharePurchaseActivity,
};
use crate::news::NewsArticle;
use crate::profile::Profile;
use crate::{
    EsgBuilder,
    core::client::RetryConfig,
    core::{CacheMode, CallOptions, DataQuality, YfClient, YfError, YfResponse},
    esg::EsgSummary,
    holders::HoldersBuilder,
    news::NewsBuilder,
};
use crate::{
    analysis::AnalysisBuilder, fundamentals::FundamentalsBuilder, history::HistoryBuilder,
};
use chrono::{DateTime, Utc};
use paft::fundamentals::analysis::{
    Earnings, EarningsTrendRow, PriceTarget, RecommendationRow, RecommendationSummary,
    UpgradeDowngradeRow,
};
use paft::fundamentals::statements::{BalanceSheetRow, CashflowRow, IncomeStatementRow};
use paft::fundamentals::statistics::KeyStatistics;
use paft::money::Currency;

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
/// let ticker = Ticker::new(&client, "TSLA");
///
/// // Get the latest quote
/// let quote = ticker.quote().await?;
/// if let Some(price) = &quote.price {
///     println!("Tesla's last price: {price:?}");
/// }
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
    options: CallOptions,
}

impl Ticker {
    /// Creates a new `Ticker` for a given symbol.
    ///
    /// This is the standard way to create a ticker instance with default API endpoints.
    pub fn new(client: &YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client: client.clone(),
            symbol: symbol.into(),
            options: CallOptions::default(),
        }
    }

    /// Sets the cache mode for all subsequent API calls made by this `Ticker` instance.
    ///
    /// This allows you to override the client's default cache behavior for a specific ticker.
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.options.cache_mode = mode;
        self
    }

    /// Overrides the client's default retry policy for all subsequent API calls made by this `Ticker` instance.
    #[must_use]
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.options = self.options.with_retry_policy(cfg);
        self
    }

    /// Sets how provider projection issues are handled for subsequent API calls.
    ///
    /// Builders created from this ticker inherit this setting.
    #[must_use]
    pub const fn data_quality(mut self, policy: DataQuality) -> Self {
        self.options.data_quality = policy;
        self
    }

    /// Fails when Yahoo data cannot be projected losslessly.
    #[must_use]
    pub const fn strict(self) -> Self {
        self.data_quality(DataQuality::Strict)
    }

    /// Fetches a comprehensive `Info` struct containing quote, profile, calendar, and analysis data.
    ///
    /// This method conveniently aggregates data from multiple endpoints into a single struct,
    /// similar to the `.info` attribute in the Python `yfinance` library. It makes several
    /// API calls concurrently to gather the data efficiently.
    ///
    /// If a non-essential part of the data fails to load (e.g., profile or analyst data),
    /// the corresponding fields in the `Info` struct will be `None`.
    ///
    /// # Errors
    ///
    /// This method will return an error if the required quote data cannot be fetched
    /// or strict data-quality mode rejects a projection issue.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err, fields(symbol = %self.symbol)))]
    pub async fn info(&self) -> Result<Info, YfError> {
        info::fetch_info(&self.client, &self.symbol, &self.options).await
    }

    /// Fetches aggregate `Info` with projection diagnostics for optional submodules.
    ///
    /// # Errors
    ///
    /// This method will return an error if the required quote data cannot be fetched
    /// or strict data-quality mode rejects a projection issue.
    pub async fn info_with_diagnostics(&self) -> Result<YfResponse<Info>, YfError> {
        info::fetch_info_with_diagnostics(&self.client, &self.symbol, &self.options).await
    }

    /// Fetches aggregate `Info` and fails if any optional submodule cannot be loaded.
    ///
    /// # Errors
    ///
    /// This method will return an error if the required quote data cannot be fetched
    /// or any optional submodule fails to project cleanly.
    pub async fn info_strict(&self) -> Result<Info, YfError> {
        let options = self.options.clone().strict();
        Ok(
            info::fetch_info_with_diagnostics(&self.client, &self.symbol, &options)
                .await?
                .into_data(),
        )
    }

    /// Fetches the company, ETF, or mutual-fund profile for the ticker.
    ///
    /// This is the high-level convenience equivalent of [`crate::profile::load_profile`],
    /// using this ticker's configured cache mode and retry policy.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or Yahoo does not provide a supported profile for the symbol.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err, fields(symbol = %self.symbol)))]
    pub async fn profile(&self) -> Result<Profile, YfError> {
        Ok(self.profile_with_diagnostics().await?.into_data())
    }

    /// Fetches the company, ETF, or mutual-fund profile with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// Yahoo does not provide a supported profile for the symbol, or strict data-quality mode
    /// rejects a projection issue.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err, fields(symbol = %self.symbol)))]
    pub async fn profile_with_diagnostics(&self) -> Result<YfResponse<Profile>, YfError> {
        crate::profile::load_profile_with_options_and_diagnostics(
            &self.client,
            &self.symbol,
            &self.options,
        )
        .await
    }

    /* ---------------- Quotes ---------------- */

    /// Fetches a detailed quote for the ticker.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err, fields(symbol = %self.symbol)))]
    pub async fn quote(&self) -> Result<Quote, YfError> {
        quote::fetch_quote(&self.client, &self.symbol, &self.options).await
    }

    /// Fetches a detailed quote with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn quote_with_diagnostics(&self) -> Result<YfResponse<Quote>, YfError> {
        quote::fetch_quote_with_diagnostics(&self.client, &self.symbol, &self.options).await
    }

    /// Fetches fast ticker information, including a quote snapshot and moving averages.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn fast_info(&self) -> Result<FastInfo, YfError> {
        quote::fetch_fast_info(&self.client, &self.symbol, &self.options).await
    }

    /// Fetches fast ticker information with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn fast_info_with_diagnostics(&self) -> Result<YfResponse<FastInfo>, YfError> {
        quote::fetch_fast_info_with_diagnostics(&self.client, &self.symbol, &self.options).await
    }

    /// Fetches valuation, dividend, volume, and risk statistics for the ticker.
    ///
    /// The values are mapped into the provider-agnostic [`KeyStatistics`] model.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn key_statistics(&self) -> Result<KeyStatistics, YfError> {
        quote::fetch_key_statistics(&self.client, &self.symbol, &self.options).await
    }

    /// Fetches valuation, dividend, volume, and risk statistics with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn key_statistics_with_diagnostics(
        &self,
    ) -> Result<YfResponse<KeyStatistics>, YfError> {
        quote::fetch_key_statistics_with_diagnostics(&self.client, &self.symbol, &self.options)
            .await
    }

    /* ---------------- News convenience ---------------- */

    /// Returns a `NewsBuilder` to construct a query for news articles.
    #[must_use]
    pub fn news_builder(&self) -> NewsBuilder {
        NewsBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.options.cache_mode())
            .data_quality(self.options.data_quality())
            .retry_policy(self.options.retry_override().cloned())
    }

    /// Fetches the latest news articles for the ticker.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn news(&self) -> Result<Vec<NewsArticle>, YfError> {
        Ok(self.news_with_diagnostics().await?.into_data())
    }

    /// Fetches the latest news articles with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn news_with_diagnostics(&self) -> Result<YfResponse<Vec<NewsArticle>>, YfError> {
        self.news_builder().fetch_with_diagnostics().await
    }

    /* ---------------- History helpers ---------------- */

    /// Returns a `HistoryBuilder` to construct a detailed query for historical price data.
    #[must_use]
    pub fn history_builder(&self) -> HistoryBuilder {
        HistoryBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.options.cache_mode())
            .data_quality(self.options.data_quality())
            .retry_policy(self.options.retry_override().cloned())
    }

    /// Fetches historical price candles with default settings.
    ///
    /// Prices are automatically adjusted for splits and dividends. For more control, use [`Self::history_builder`].
    ///
    /// # Arguments
    /// * `range` - The relative time range for the data (e.g., `1y`, `6mo`). Defaults to `6mo` if `None`.
    /// * `interval` - The time interval for each candle (e.g., `1d`, `1wk`). Defaults to `1d` if `None`.
    /// * `prepost` - Whether to include pre-market and post-market data for intraday intervals.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err, fields(symbol = %self.symbol)))]
    pub async fn history(
        &self,
        range: Option<Range>,
        interval: Option<Interval>,
        prepost: bool,
    ) -> Result<Vec<Candle>, crate::core::YfError> {
        Ok(self
            .history_with_diagnostics(range, interval, prepost)
            .await?
            .into_data())
    }

    /// Fetches historical price candles with projection diagnostics.
    ///
    /// Prices are automatically adjusted for splits and dividends. For more control, use [`Self::history_builder`].
    ///
    /// # Arguments
    /// * `range` - The relative time range for the data (e.g., `1y`, `6mo`). Defaults to `6mo` if `None`.
    /// * `interval` - The time interval for each candle (e.g., `1d`, `1wk`). Defaults to `1d` if `None`.
    /// * `prepost` - Whether to include pre-market and post-market data for intraday intervals.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn history_with_diagnostics(
        &self,
        range: Option<Range>,
        interval: Option<Interval>,
        prepost: bool,
    ) -> Result<YfResponse<Vec<Candle>>, YfError> {
        let mut hb = self.history_builder();
        if let Some(r) = range {
            hb = hb.range(r);
        }
        if let Some(i) = interval {
            hb = hb.interval(i);
        }
        hb = hb.auto_adjust(true).prepost(prepost).actions(true);
        hb.fetch_with_diagnostics().await
    }

    /// Fetches all corporate actions (dividends, splits, and capital gains) for the given range.
    ///
    /// Defaults to the maximum available range if `None`.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err, fields(symbol = %self.symbol)))]
    pub async fn actions(&self, range: Option<Range>) -> Result<Vec<Action>, YfError> {
        Ok(self.actions_with_diagnostics(range).await?.into_data())
    }

    /// Fetches all corporate actions with projection diagnostics.
    ///
    /// Defaults to the maximum available range if `None`.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn actions_with_diagnostics(
        &self,
        range: Option<Range>,
    ) -> Result<YfResponse<Vec<Action>>, YfError> {
        let mut hb = self.history_builder();
        hb = hb.range(range.unwrap_or(Range::Max));
        Ok(hb
            .auto_adjust(true)
            .actions(true)
            .fetch_full_with_diagnostics()
            .await?
            .map(|resp| {
                let mut actions = resp.actions;
                actions.sort_by_key(action_sort_key);
                actions
            }))
    }

    /// Fetches the metadata associated with the ticker's historical data, such as timezone.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), err, fields(symbol = %self.symbol)))]
    pub async fn get_history_metadata(
        &self,
        range: Option<Range>,
    ) -> Result<Option<HistoryMeta>, crate::core::YfError> {
        Ok(self
            .get_history_metadata_with_diagnostics(range)
            .await?
            .into_data())
    }

    /// Fetches the ticker's historical metadata with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn get_history_metadata_with_diagnostics(
        &self,
        range: Option<Range>,
    ) -> Result<YfResponse<Option<HistoryMeta>>, YfError> {
        let mut hb = self.history_builder();
        if let Some(r) = range {
            hb = hb.range(r);
        }
        Ok(hb
            .fetch_full_with_diagnostics()
            .await?
            .map(|resp| resp.meta))
    }

    /// Fetches the ISIN for the ticker by searching on markets.businessinsider.com.
    ///
    /// This mimics the approach used by the Python `yfinance` library.
    /// It returns `None` for assets that don't have an ISIN, such as indices.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn isin(&self) -> Result<Option<String>, YfError> {
        let symbol = crate::core::client::normalize_symbol(&self.symbol)?;
        if symbol.contains('^') {
            return Ok(None);
        }

        isin::fetch_isin(&self.client, &symbol, self.options.retry_override()).await
    }

    /// Fetches the member symbols of an index (for example `^FCHI` returns `OR.PA`, `MC.PA`, ...).
    ///
    /// Uses Yahoo's `components` quoteSummary module. Symbols that have no components,
    /// such as individual equities or ETFs, return an empty list. Yahoo answers unknown
    /// symbols the same way, so they also return an empty list rather than an error.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn components(&self) -> Result<Vec<String>, YfError> {
        components::fetch_components(&self.client, &self.symbol, &self.options).await
    }

    /* ---------------- Options ---------------- */

    /// Fetches the available expiration dates for the ticker's options as Unix timestamps.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn options(&self) -> Result<Vec<i64>, YfError> {
        options::expiration_dates(&self.client, &self.symbol, &self.options).await
    }

    /// Fetches the full option chain (calls and puts) for a specific expiration date.
    ///
    /// If `date` is `None`, fetches the chain for the nearest expiration date.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn option_chain(&self, date: Option<i64>) -> Result<OptionChain, YfError> {
        options::option_chain(&self.client, &self.symbol, date, &self.options).await
    }

    /// Fetches the full option chain with projection diagnostics.
    ///
    /// If `date` is `None`, fetches the chain for the nearest expiration date.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn option_chain_with_diagnostics(
        &self,
        date: Option<i64>,
    ) -> Result<YfResponse<OptionChain>, YfError> {
        options::option_chain_with_diagnostics(&self.client, &self.symbol, date, &self.options)
            .await
    }

    /* ---------------- Holders convenience ---------------- */

    fn holders_builder(&self) -> HoldersBuilder {
        HoldersBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.options.cache_mode())
            .data_quality(self.options.data_quality())
            .retry_policy(self.options.retry_override().cloned())
    }

    /// Fetches the major holders breakdown (e.g., % insiders, % institutions).
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn major_holders(&self) -> Result<Vec<MajorHolder>, YfError> {
        Ok(self.major_holders_with_diagnostics().await?.into_data())
    }

    /// Fetches the major holders breakdown with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn major_holders_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Vec<MajorHolder>>, YfError> {
        self.holders_builder()
            .major_holders_with_diagnostics()
            .await
    }

    /// Fetches a list of the top institutional holders.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn institutional_holders(&self) -> Result<Vec<InstitutionalHolder>, YfError> {
        Ok(self
            .institutional_holders_with_diagnostics()
            .await?
            .into_data())
    }

    /// Fetches institutional holders with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn institutional_holders_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Vec<InstitutionalHolder>>, YfError> {
        self.holders_builder()
            .institutional_holders_with_diagnostics()
            .await
    }

    /// Fetches a list of the top mutual fund holders.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn mutual_fund_holders(&self) -> Result<Vec<InstitutionalHolder>, YfError> {
        Ok(self
            .mutual_fund_holders_with_diagnostics()
            .await?
            .into_data())
    }

    /// Fetches mutual fund holders with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn mutual_fund_holders_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Vec<InstitutionalHolder>>, YfError> {
        self.holders_builder()
            .mutual_fund_holders_with_diagnostics()
            .await
    }

    /// Fetches a list of recent insider transactions.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn insider_transactions(&self) -> Result<Vec<InsiderTransaction>, YfError> {
        Ok(self
            .insider_transactions_with_diagnostics()
            .await?
            .into_data())
    }

    /// Fetches insider transactions with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn insider_transactions_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Vec<InsiderTransaction>>, YfError> {
        self.holders_builder()
            .insider_transactions_with_diagnostics()
            .await
    }

    /// Fetches a roster of company insiders and their holdings.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn insider_roster_holders(&self) -> Result<Vec<InsiderRosterHolder>, YfError> {
        Ok(self
            .insider_roster_holders_with_diagnostics()
            .await?
            .into_data())
    }

    /// Fetches insider roster holders with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn insider_roster_holders_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Vec<InsiderRosterHolder>>, YfError> {
        self.holders_builder()
            .insider_roster_holders_with_diagnostics()
            .await
    }

    /// Fetches a summary of net insider purchase and sale activity.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn net_share_purchase_activity(
        &self,
    ) -> Result<Option<NetSharePurchaseActivity>, YfError> {
        Ok(self
            .net_share_purchase_activity_with_diagnostics()
            .await?
            .into_data())
    }

    /// Fetches net insider purchase and sale activity with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn net_share_purchase_activity_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Option<NetSharePurchaseActivity>>, YfError> {
        self.holders_builder()
            .net_share_purchase_activity_with_diagnostics()
            .await
    }

    /* ---------------- Analysis convenience ---------------- */

    fn analysis_builder(&self) -> AnalysisBuilder {
        AnalysisBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.options.cache_mode())
            .data_quality(self.options.data_quality())
            .retry_policy(self.options.retry_override().cloned())
    }

    /// Fetches the analyst recommendation trend.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn recommendations(&self) -> Result<Vec<RecommendationRow>, YfError> {
        Ok(self.recommendations_with_diagnostics().await?.into_data())
    }

    /// Fetches analyst recommendation trends with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn recommendations_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Vec<RecommendationRow>>, YfError> {
        self.analysis_builder()
            .recommendations_with_diagnostics()
            .await
    }

    /// Fetches a summary of the latest analyst recommendations.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn recommendations_summary(&self) -> Result<RecommendationSummary, YfError> {
        Ok(self
            .recommendations_summary_with_diagnostics()
            .await?
            .into_data())
    }

    /// Fetches the latest analyst recommendation summary with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn recommendations_summary_with_diagnostics(
        &self,
    ) -> Result<YfResponse<RecommendationSummary>, YfError> {
        self.analysis_builder()
            .recommendations_summary_with_diagnostics()
            .await
    }

    /// Fetches the history of analyst upgrades and downgrades.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn upgrades_downgrades(&self) -> Result<Vec<UpgradeDowngradeRow>, YfError> {
        Ok(self
            .upgrades_downgrades_with_diagnostics()
            .await?
            .into_data())
    }

    /// Fetches analyst upgrades and downgrades with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn upgrades_downgrades_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Vec<UpgradeDowngradeRow>>, YfError> {
        self.analysis_builder()
            .upgrades_downgrades_with_diagnostics()
            .await
    }

    /// Fetches the analyst price target.
    ///
    /// Provide `Some(currency)` to override the auto-resolved currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn analyst_price_target(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<PriceTarget, YfError> {
        Ok(self
            .analyst_price_target_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches analyst price targets with projection diagnostics.
    ///
    /// Provide `Some(currency)` to override the auto-resolved currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn analyst_price_target_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<PriceTarget>, YfError> {
        self.analysis_builder()
            .analyst_price_target_with_diagnostics(override_currency)
            .await
    }

    /// Fetches earnings trend data for the ticker.
    ///
    /// This includes earnings estimates, revenue estimates, EPS trends, and EPS revisions for various periods.
    ///
    /// Provide `Some(currency)` to override the auto-resolved currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn earnings_trend(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<Vec<EarningsTrendRow>, YfError> {
        Ok(self
            .earnings_trend_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches earnings trend data with projection diagnostics.
    ///
    /// This includes earnings estimates, revenue estimates, EPS trends, and EPS revisions for various periods.
    ///
    /// Provide `Some(currency)` to override the auto-resolved currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn earnings_trend_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<Vec<EarningsTrendRow>>, YfError> {
        self.analysis_builder()
            .earnings_trend_with_diagnostics(override_currency)
            .await
    }

    /* ---------------- ESG / Sustainability ---------------- */

    fn esg_builder(&self) -> EsgBuilder {
        EsgBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.options.cache_mode())
            .data_quality(self.options.data_quality())
            .retry_policy(self.options.retry_override().cloned())
    }

    /// Fetches ESG (Environmental, Social, Governance) scores and involvement data for the ticker.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn sustainability(&self) -> Result<EsgSummary, YfError> {
        self.esg_builder().fetch().await
    }

    /// Fetches ESG scores and involvement data with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode rejects a projection issue.
    pub async fn sustainability_with_diagnostics(&self) -> Result<YfResponse<EsgSummary>, YfError> {
        self.esg_builder().fetch_with_diagnostics().await
    }
    /* ---------------- Fundamentals convenience ---------------- */

    fn fundamentals_builder(&self) -> FundamentalsBuilder {
        FundamentalsBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.options.cache_mode())
            .data_quality(self.options.data_quality())
            .retry_policy(self.options.retry_override().cloned())
    }

    /// Fetches the annual income statement.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn income_stmt(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<Vec<IncomeStatementRow>, YfError> {
        Ok(self
            .income_stmt_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches the annual income statement with projection diagnostics.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn income_stmt_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<Vec<IncomeStatementRow>>, YfError> {
        self.fundamentals_builder()
            .income_statement_with_diagnostics(false, override_currency)
            .await
    }

    /// Fetches the quarterly income statement.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn quarterly_income_stmt(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<Vec<IncomeStatementRow>, YfError> {
        Ok(self
            .quarterly_income_stmt_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches the quarterly income statement with projection diagnostics.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn quarterly_income_stmt_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<Vec<IncomeStatementRow>>, YfError> {
        self.fundamentals_builder()
            .income_statement_with_diagnostics(true, override_currency)
            .await
    }

    /// Fetches the annual balance sheet.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn balance_sheet(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<Vec<BalanceSheetRow>, YfError> {
        Ok(self
            .balance_sheet_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches the annual balance sheet with projection diagnostics.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn balance_sheet_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<Vec<BalanceSheetRow>>, YfError> {
        self.fundamentals_builder()
            .balance_sheet_with_diagnostics(false, override_currency)
            .await
    }

    /// Fetches the quarterly balance sheet.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn quarterly_balance_sheet(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<Vec<BalanceSheetRow>, YfError> {
        Ok(self
            .quarterly_balance_sheet_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches the quarterly balance sheet with projection diagnostics.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn quarterly_balance_sheet_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<Vec<BalanceSheetRow>>, YfError> {
        self.fundamentals_builder()
            .balance_sheet_with_diagnostics(true, override_currency)
            .await
    }

    /// Fetches the annual cash flow statement.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn cashflow(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<Vec<CashflowRow>, YfError> {
        Ok(self
            .cashflow_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches the annual cash flow statement with projection diagnostics.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn cashflow_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<Vec<CashflowRow>>, YfError> {
        self.fundamentals_builder()
            .cashflow_with_diagnostics(false, override_currency)
            .await
    }

    /// Fetches the quarterly cash flow statement.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn quarterly_cashflow(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<Vec<CashflowRow>, YfError> {
        Ok(self
            .quarterly_cashflow_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches the quarterly cash flow statement with projection diagnostics.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn quarterly_cashflow_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<Vec<CashflowRow>>, YfError> {
        self.fundamentals_builder()
            .cashflow_with_diagnostics(true, override_currency)
            .await
    }

    /// Fetches earnings history and estimates.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn earnings(&self, override_currency: Option<Currency>) -> Result<Earnings, YfError> {
        Ok(self
            .earnings_with_diagnostics(override_currency)
            .await?
            .into_data())
    }

    /// Fetches earnings history and estimates with projection diagnostics.
    ///
    /// Provide `Some(currency)` to override the auto-resolved reporting currency for this call;
    /// pass `None` to enrich currency metadata from Yahoo and infer only when needed.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn earnings_with_diagnostics(
        &self,
        override_currency: Option<Currency>,
    ) -> Result<YfResponse<Earnings>, YfError> {
        self.fundamentals_builder()
            .earnings_with_diagnostics(override_currency)
            .await
    }

    /// Fetches corporate calendar events like earnings dates.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn calendar(&self) -> Result<Calendar, YfError> {
        Ok(self.calendar_with_diagnostics().await?.into_data())
    }

    /// Fetches corporate calendar events with projection diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn calendar_with_diagnostics(&self) -> Result<YfResponse<Calendar>, YfError> {
        self.fundamentals_builder()
            .calendar_with_diagnostics()
            .await
    }

    /// Fetches historical annual shares outstanding.
    ///
    /// This convenience method uses Yahoo's rolling 548-day share-count window, matching
    /// Python yfinance's `get_shares_full(start=None, end=None)`. Use
    /// [`Self::shares_between`] to request an explicit wider window.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn shares(&self) -> Result<Vec<ShareCount>, YfError> {
        Ok(self.shares_with_diagnostics().await?.into_data())
    }

    /// Fetches historical annual shares outstanding with projection diagnostics.
    ///
    /// This convenience method uses Yahoo's rolling 548-day share-count window, matching
    /// Python yfinance's `get_shares_full(start=None, end=None)`. Use
    /// [`Self::shares_between_with_diagnostics`] to request an explicit wider window.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn shares_with_diagnostics(&self) -> Result<YfResponse<Vec<ShareCount>>, YfError> {
        self.fundamentals_builder()
            .shares_with_diagnostics(false)
            .await
    }

    /// Fetches historical annual shares outstanding within an explicit UTC time window.
    ///
    /// # Errors
    ///
    /// This method will return an error if the date range is invalid, the request fails,
    /// or the response cannot be parsed.
    pub async fn shares_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ShareCount>, YfError> {
        Ok(self
            .shares_between_with_diagnostics(start, end)
            .await?
            .into_data())
    }

    /// Fetches historical annual shares outstanding within an explicit UTC time window with diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the date range is invalid, the request fails,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn shares_between_with_diagnostics(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<YfResponse<Vec<ShareCount>>, YfError> {
        self.fundamentals_builder()
            .shares_between_with_diagnostics(false, start, end)
            .await
    }

    /// Fetches historical quarterly shares outstanding.
    ///
    /// This convenience method uses Yahoo's rolling 548-day share-count window, matching
    /// Python yfinance's `get_shares_full(start=None, end=None)`. Use
    /// [`Self::quarterly_shares_between`] to request an explicit wider window.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn quarterly_shares(&self) -> Result<Vec<ShareCount>, YfError> {
        Ok(self.quarterly_shares_with_diagnostics().await?.into_data())
    }

    /// Fetches historical quarterly shares outstanding with projection diagnostics.
    ///
    /// This convenience method uses Yahoo's rolling 548-day share-count window, matching
    /// Python yfinance's `get_shares_full(start=None, end=None)`. Use
    /// [`Self::quarterly_shares_between_with_diagnostics`] to request an explicit wider window.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or strict data-quality mode
    /// rejects a projection issue.
    pub async fn quarterly_shares_with_diagnostics(
        &self,
    ) -> Result<YfResponse<Vec<ShareCount>>, YfError> {
        self.fundamentals_builder()
            .shares_with_diagnostics(true)
            .await
    }

    /// Fetches historical quarterly shares outstanding within an explicit UTC time window.
    ///
    /// # Errors
    ///
    /// This method will return an error if the date range is invalid, the request fails,
    /// or the response cannot be parsed.
    pub async fn quarterly_shares_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ShareCount>, YfError> {
        Ok(self
            .quarterly_shares_between_with_diagnostics(start, end)
            .await?
            .into_data())
    }

    /// Fetches historical quarterly shares outstanding within an explicit UTC time window with diagnostics.
    ///
    /// # Errors
    ///
    /// This method will return an error if the date range is invalid, the request fails,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn quarterly_shares_between_with_diagnostics(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<YfResponse<Vec<ShareCount>>, YfError> {
        self.fundamentals_builder()
            .shares_between_with_diagnostics(true, start, end)
            .await
    }
}

const fn action_sort_key(action: &Action) -> (bool, chrono::NaiveDate) {
    match action {
        Action::Dividend { date, .. }
        | Action::Split { date, .. }
        | Action::CapitalGain { date, .. } => (false, *date),
        _ => (true, chrono::NaiveDate::MAX),
    }
}
