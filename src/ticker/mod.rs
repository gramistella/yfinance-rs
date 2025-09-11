mod info;
mod isin;
mod model;
mod options;
mod quote;

pub use model::{FastInfo, Info, OptionChain, OptionContract};

use crate::{
    EsgBuilder, HoldersBuilder, NewsBuilder, YfClient, YfError,
    analysis::AnalysisBuilder,
    core::client::{CacheMode, RetryConfig},
    core::conversions::*,
    fundamentals::FundamentalsBuilder,
    history::HistoryBuilder,
};
use paft::prelude::*;
use paft::fundamentals::{
    MajorHolder, InstitutionalHolder, InsiderTransaction, InsiderRosterHolder, 
    NetSharePurchaseActivity, RecommendationRow, RecommendationSummary, 
    UpgradeDowngradeRow, PriceTarget, EarningsTrendRow, EsgScores,
    IncomeStatementRow, BalanceSheetRow, CashflowRow, Earnings, Calendar, ShareCount
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
/// let ticker = Ticker::new(&client, "TSLA");
///
/// // Get the latest quote
/// let quote = ticker.quote().await?;
/// println!("Tesla's last price: {}", quote.price.as_ref().map(|p| yfinance_rs::core::conversions::money_to_f64(p)).unwrap_or(0.0));
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
    pub fn new(client: &YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client: client.clone(),
            symbol: symbol.into(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for all subsequent API calls made by this `Ticker` instance.
    ///
    /// This allows you to override the client's default cache behavior for a specific ticker.
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the client's default retry policy for all subsequent API calls made by this `Ticker` instance.
    #[must_use]
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Fetches a comprehensive `Info` struct containing quote, profile, analysis, and ESG data.
    ///
    /// This method conveniently aggregates data from multiple endpoints into a single struct,
    /// similar to the `.info` attribute in the Python `yfinance` library. It makes several
    /// API calls concurrently to gather the data efficiently.
    ///
    /// If a non-essential part of the data fails to load (e.g., ESG scores), the corresponding
    /// fields in the `Info` struct will be `None`. A failure to load the core profile
    /// will result in an error.
    ///
    /// # Errors
    ///
    /// This method will return an error if the core profile data cannot be fetched.
    pub async fn info(&self) -> Result<Info, YfError> {
        info::fetch_info(
            &self.client,
            &self.symbol,
            self.cache_mode,
            self.retry_override.as_ref(),
        )
        .await
    }

    /* ---------------- Quotes ---------------- */

    /// Fetches a detailed quote for the ticker.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
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
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails, the response cannot be parsed,
    /// or if the last/previous price is not available in the quote.
    pub async fn fast_info(&self) -> Result<FastInfo, YfError> {
        let q = self.quote().await?;
        let last = q
            .price
            .as_ref()
            .map(money_to_f64)
            .or(q.previous_close.as_ref().map(money_to_f64))
            .ok_or_else(|| YfError::MissingData("quote missing last/previous price".into()))?;

        Ok(FastInfo {
            symbol: q.symbol,
            last_price: last,
            previous_close: q.previous_close.as_ref().map(money_to_f64),
            currency: None, // paft Quote doesn't have currency field
            exchange: exchange_to_string(q.exchange),
            market_state: market_state_to_string(q.market_state),
        })
    }

    /* ---------------- News convenience ---------------- */

    /// Returns a `NewsBuilder` to construct a query for news articles.
    #[must_use]
    pub fn news_builder(&self) -> NewsBuilder {
        NewsBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the latest news articles for the ticker.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn news(&self) -> Result<Vec<crate::NewsArticle>, YfError> {
        self.news_builder().fetch().await
    }

    /* ---------------- History helpers ---------------- */

    /// Returns a `HistoryBuilder` to construct a detailed query for historical price data.
    #[must_use]
    pub fn history_builder(&self) -> HistoryBuilder {
        HistoryBuilder::new(&self.client, &self.symbol)
    }

    /// Fetches historical price candles with default settings.
    ///
    /// Prices are automatically adjusted for splits and dividends. For more control, use [`history_builder`].
    ///
    /// # Arguments
    /// * `range` - The relative time range for the data (e.g., `1y`, `6mo`). Defaults to `6mo` if `None`.
    /// * `interval` - The time interval for each candle (e.g., `1d`, `1wk`). Defaults to `1d` if `None`.
    /// * `prepost` - Whether to include pre-market and post-market data for intraday intervals.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
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
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn actions(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<crate::Action>, YfError> {
        let mut hb = self.history_builder();
        hb = hb.range(range.unwrap_or(crate::Range::Max));
        let resp = hb.auto_adjust(true).actions(true).fetch_full().await?;
        let mut actions = resp.actions;
        actions.sort_by_key(|a| match *a {
            crate::Action::Dividend { ts, .. }
            | crate::Action::Split { ts, .. }
            | crate::Action::CapitalGain { ts, .. } => ts,
        });
        Ok(actions)
    }

    /// Fetches all dividend payments for the given range.
    ///
    /// Returns a `Vec` of tuples containing `(timestamp, amount)`.
    /// Defaults to the maximum available range if `None`.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn dividends(&self, range: Option<crate::Range>) -> Result<Vec<(i64, f64)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Dividend { ts, amount } => Some((datetime_to_i64(ts), money_to_f64(&amount))),
                _ => None,
            })
            .collect())
    }

    /// Fetches all stock splits for the given range.
    ///
    /// Returns a `Vec` of tuples containing `(timestamp, numerator, denominator)`.
    /// Defaults to the maximum available range if `None`.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
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
                } => Some((datetime_to_i64(ts), numerator, denominator)),
                _ => None,
            })
            .collect())
    }

    /// Fetches the metadata associated with the ticker's historical data, such as timezone.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
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

    /// Fetches the ISIN for the ticker by searching on markets.businessinsider.com.
    ///
    /// This mimics the approach used by the Python `yfinance` library.
    /// It returns `None` for assets that don't have an ISIN, such as indices.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn isin(&self) -> Result<Option<String>, YfError> {
        if self.symbol.contains('^') {
            return Ok(None);
        }

        isin::fetch_isin(&self.client, &self.symbol, self.retry_override.as_ref()).await
    }

    /// Retrieves historical capital gain events for the ticker (typically for mutual funds).
    ///
    /// A time `range` can be optionally specified. Defaults to the maximum available range.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn capital_gains(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<(i64, f64)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::CapitalGain { ts, gain } => Some((datetime_to_i64(ts), money_to_f64(&gain))),
                _ => None,
            })
            .collect())
    }

    /* ---------------- Options ---------------- */

    /// Fetches the available expiration dates for the ticker's options as Unix timestamps.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
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
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
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

    /* ---------------- Holders convenience ---------------- */

    fn holders_builder(&self) -> HoldersBuilder {
        HoldersBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the major holders breakdown (e.g., % insiders, % institutions).
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn major_holders(&self) -> Result<Vec<MajorHolder>, YfError> {
        self.holders_builder().major_holders().await
    }

    /// Fetches a list of the top institutional holders.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn institutional_holders(&self) -> Result<Vec<InstitutionalHolder>, YfError> {
        self.holders_builder().institutional_holders().await
    }

    /// Fetches a list of the top mutual fund holders.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn mutual_fund_holders(&self) -> Result<Vec<InstitutionalHolder>, YfError> {
        self.holders_builder().mutual_fund_holders().await
    }

    /// Fetches a list of recent insider transactions.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn insider_transactions(&self) -> Result<Vec<InsiderTransaction>, YfError> {
        self.holders_builder().insider_transactions().await
    }

    /// Fetches a roster of company insiders and their holdings.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn insider_roster_holders(&self) -> Result<Vec<InsiderRosterHolder>, YfError> {
        self.holders_builder().insider_roster_holders().await
    }

    /// Fetches a summary of net insider purchase and sale activity.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn net_share_purchase_activity(
        &self,
    ) -> Result<Option<NetSharePurchaseActivity>, YfError> {
        self.holders_builder().net_share_purchase_activity().await
    }

    /* ---------------- Analysis convenience ---------------- */

    fn analysis_builder(&self) -> AnalysisBuilder {
        AnalysisBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the analyst recommendation trend.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn recommendations(&self) -> Result<Vec<RecommendationRow>, YfError> {
        self.analysis_builder().recommendations().await
    }

    /// Fetches a summary of the latest analyst recommendations.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn recommendations_summary(&self) -> Result<RecommendationSummary, YfError> {
        self.analysis_builder().recommendations_summary().await
    }

    /// Fetches the history of analyst upgrades and downgrades.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn upgrades_downgrades(&self) -> Result<Vec<UpgradeDowngradeRow>, YfError> {
        self.analysis_builder().upgrades_downgrades().await
    }

    /// Fetches the analyst price target.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn analyst_price_target(&self) -> Result<PriceTarget, YfError> {
        self.analysis_builder().analyst_price_target().await
    }

    /// Fetches earnings trend data for the ticker.
    ///
    /// This includes earnings estimates, revenue estimates, EPS trends, and EPS revisions for various periods
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn earnings_trend(&self) -> Result<Vec<EarningsTrendRow>, YfError> {
        self.analysis_builder().earnings_trend().await
    }

    /* ---------------- ESG / Sustainability ---------------- */

    fn esg_builder(&self) -> EsgBuilder {
        EsgBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the ESG (Environmental, Social, Governance) scores for the ticker.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn sustainability(&self) -> Result<EsgScores, YfError> {
        self.esg_builder().fetch().await
    }
    /* ---------------- Fundamentals convenience ---------------- */

    fn fundamentals_builder(&self) -> FundamentalsBuilder {
        FundamentalsBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the annual income statement.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn income_stmt(&self) -> Result<Vec<IncomeStatementRow>, YfError> {
        self.fundamentals_builder().income_statement(false).await
    }

    /// Fetches the quarterly income statement.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn quarterly_income_stmt(&self) -> Result<Vec<IncomeStatementRow>, YfError> {
        self.fundamentals_builder().income_statement(true).await
    }

    /// Fetches the annual balance sheet.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn balance_sheet(&self) -> Result<Vec<BalanceSheetRow>, YfError> {
        self.fundamentals_builder().balance_sheet(false).await
    }

    /// Fetches the quarterly balance sheet.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn quarterly_balance_sheet(&self) -> Result<Vec<BalanceSheetRow>, YfError> {
        self.fundamentals_builder().balance_sheet(true).await
    }

    /// Fetches the annual cash flow statement.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn cashflow(&self) -> Result<Vec<CashflowRow>, YfError> {
        self.fundamentals_builder().cashflow(false).await
    }

    /// Fetches the quarterly cash flow statement.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn quarterly_cashflow(&self) -> Result<Vec<CashflowRow>, YfError> {
        self.fundamentals_builder().cashflow(true).await
    }

    /// Fetches earnings history and estimates.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn earnings(&self) -> Result<Earnings, YfError> {
        self.fundamentals_builder().earnings().await
    }

    /// Fetches corporate calendar events like earnings dates.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn calendar(&self) -> Result<Calendar, YfError> {
        self.fundamentals_builder().calendar().await
    }

    /// Fetches historical annual shares outstanding.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn shares(&self) -> Result<Vec<ShareCount>, YfError> {
        self.fundamentals_builder().shares(false).await
    }

    /// Fetches historical quarterly shares outstanding.
    ///
    /// # Errors
    ///
    /// This method will return an error if the request fails or the response cannot be parsed.
    pub async fn quarterly_shares(&self) -> Result<Vec<ShareCount>, YfError> {
        self.fundamentals_builder().shares(true).await
    }
}
