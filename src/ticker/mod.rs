mod info;
mod model;
mod options;
mod quote;

pub use info::Info;
pub use model::{FastInfo, OptionChain, OptionContract};

use crate::{
    EsgBuilder, HoldersBuilder, NewsBuilder, Quote, ShareCount, YfClient, YfError,
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

    /// Fetches a comprehensive `Info` struct containing quote, profile, analysis, and ESG data.
    ///
    /// This method conveniently aggregates data from multiple endpoints into a single struct,
    /// similar to the `.info` attribute in the Python `yfinance` library. It makes several
    /// API calls concurrently to gather the data efficiently.
    ///
    /// If a non-essential part of the data fails to load (e.g., ESG scores), the corresponding
    /// fields in the `Info` struct will be `None`. A failure to load the core profile
    /// will result in an error.
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

    /* ---------------- News convenience ---------------- */

    /// Returns a `NewsBuilder` to construct a query for news articles.
    pub fn news_builder(&self) -> NewsBuilder {
        NewsBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the latest news articles for the ticker.
    pub async fn news(&self) -> Result<Vec<crate::NewsArticle>, YfError> {
        self.news_builder().fetch().await
    }

    /* ---------------- History helpers ---------------- */

    /// Returns a `HistoryBuilder` to construct a detailed query for historical price data.
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

    /// Fetches the ISIN for the ticker by searching on markets.businessinsider.com.
    ///
    /// This mimics the approach used by the Python `yfinance` library.
    /// It returns `None` for assets that don't have an ISIN, such as indices.
    pub async fn isin(&self) -> Result<Option<String>, YfError> {
        if self.symbol.contains('^') {
            return Ok(None);
        }

        fetch_and_parse_isin(
            &self.client,
            &self.symbol,
            &self.symbol,
            self.retry_override.as_ref(),
        )
        .await
    }

    /// Retrieves historical capital gain events for the ticker (typically for mutual funds).
    ///
    /// A time `range` can be optionally specified. Defaults to the maximum available range.
    pub async fn capital_gains(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<(i64, f64)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::CapitalGain { ts, gain } => Some((ts, gain)),
                _ => None,
            })
            .collect())
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

    /* ---------------- Holders convenience ---------------- */

    fn holders_builder(&self) -> HoldersBuilder {
        HoldersBuilder::new(self.client.clone(), &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the major holders breakdown (e.g., % insiders, % institutions).
    pub async fn major_holders(&self) -> Result<Vec<crate::MajorHolder>, YfError> {
        self.holders_builder().major_holders().await
    }

    /// Fetches a list of the top institutional holders.
    pub async fn institutional_holders(&self) -> Result<Vec<crate::InstitutionalHolder>, YfError> {
        self.holders_builder().institutional_holders().await
    }

    /// Fetches a list of the top mutual fund holders.
    pub async fn mutual_fund_holders(&self) -> Result<Vec<crate::InstitutionalHolder>, YfError> {
        self.holders_builder().mutual_fund_holders().await
    }

    /// Fetches a list of recent insider transactions.
    pub async fn insider_transactions(&self) -> Result<Vec<crate::InsiderTransaction>, YfError> {
        self.holders_builder().insider_transactions().await
    }

    /// Fetches a roster of company insiders and their holdings.
    pub async fn insider_roster_holders(&self) -> Result<Vec<crate::InsiderRosterHolder>, YfError> {
        self.holders_builder().insider_roster_holders().await
    }

    /// Fetches a summary of net insider purchase and sale activity.
    pub async fn net_share_purchase_activity(
        &self,
    ) -> Result<Option<crate::NetSharePurchaseActivity>, YfError> {
        self.holders_builder().net_share_purchase_activity().await
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

    /// Fetches earnings trend data for the ticker.
    ///
    /// This includes earnings estimates, revenue estimates, EPS trends, and EPS revisions for various periods
    pub async fn earnings_trend(&self) -> Result<Vec<crate::EarningsTrendRow>, YfError> {
        self.analysis_builder().earnings_trend().await
    }

    /* ---------------- ESG / Sustainability ---------------- */

    fn esg_builder(&self) -> EsgBuilder {
        EsgBuilder::new(&self.client, &self.symbol)
            .cache_mode(self.cache_mode)
            .retry_policy(self.retry_override.clone())
    }

    /// Fetches the ESG (Environmental, Social, Governance) scores for the ticker.
    pub async fn sustainability(&self) -> Result<crate::EsgScores, YfError> {
        self.esg_builder().fetch().await
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

    /// Fetches historical annual shares outstanding.
    pub async fn shares(&self) -> Result<Vec<ShareCount>, YfError> {
        self.fundamentals_builder().shares(false).await
    }

    /// Fetches historical quarterly shares outstanding.
    pub async fn quarterly_shares(&self) -> Result<Vec<ShareCount>, YfError> {
        self.fundamentals_builder().shares(true).await
    }
}

async fn fetch_and_parse_isin(
    client: &YfClient,
    symbol: &str,
    query: &str,
    retry_override: Option<&RetryConfig>,
) -> Result<Option<String>, YfError> {
    
    #[derive(serde::Deserialize)]
    struct FlatSuggest {
        #[serde(alias = "Value", alias = "value")]
        value: Option<String>,
        #[serde(alias = "Symbol", alias = "symbol")]
        symbol: Option<String>,
        #[serde(alias = "Isin", alias = "isin", alias = "ISIN")]
        isin: Option<String>,
    }
    
    let mut url = client.base_insider_search().clone();
    url.query_pairs_mut()
        .append_pair("max_results", "5")
        .append_pair("query", query);

    let req = client.http().get(url.clone());
    let resp = client.send_with_retry(req, retry_override).await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let body = crate::core::net::get_text(resp, "isin_search", symbol, "json").await?;
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");

    // ---- Helpers ----
    let normalize_sym = |s: &str| {
        let mut t = s.trim().replace('-', ".");
        for sep in ['.', ':', ' ', '\t', '\n', '\r'] {
            if let Some(idx) = t.find(sep) {
                t.truncate(idx);
                break;
            }
        }
        t.to_ascii_lowercase()
    };

    let looks_like_isin = |s: &str| {
        let t = s.trim();
        if t.len() != 12 {
            return false;
        }
        let b = t.as_bytes();
        if !(b[0].is_ascii_alphabetic() && b[1].is_ascii_alphabetic()) {
            return false;
        }
        if !t[2..11].chars().all(|c| c.is_ascii_alphanumeric()) {
            return false;
        }
        b[11].is_ascii_digit()
    };

    let pick_from_parts = |parts: &[String], target_norm: &str| -> Option<String> {
        if let Some(first) = parts.first()
            && normalize_sym(first) == target_norm
        {
            // NOTE: parts.iter() yields &String; find gets &&String; deref once.
            if let Some(isin) = parts
                .iter()
                .map(std::string::String::as_str)
                .find(|s| looks_like_isin(s))
            {
                return Some(isin.to_uppercase());
            }
        }
        None
    };

    let extract_from_json_value = |v: &serde_json::Value, target_norm: &str| -> Option<String> {
        let mut arrays: Vec<&serde_json::Value> = Vec::new();

        match v {
            serde_json::Value::Array(_) => arrays.push(v),
            serde_json::Value::Object(map) => {
                for key in [
                    "Suggestions",
                    "suggestions",
                    "items",
                    "results",
                    "Result",
                    "data",
                ] {
                    if let Some(val) = map.get(key)
                        && val.is_array()
                    {
                        arrays.push(val);
                    }
                }
                if arrays.is_empty() {
                    for (_, val) in map {
                        if val.is_array() {
                            arrays.push(val);
                        } else if let Some(obj) = val.as_object() {
                            for (_, inner) in obj {
                                if inner.is_array() {
                                    arrays.push(inner);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        for arr in arrays {
            if let Some(a) = arr.as_array() {
                for item in a {
                    if let Some(obj) = item.as_object() {
                        // Direct ISIN fields
                        for k in ["Isin", "isin", "ISIN"] {
                            if let Some(isin_val) = obj.get(k).and_then(|x| x.as_str())
                                && looks_like_isin(isin_val)
                            {
                                let sym = obj
                                    .get("Symbol")
                                    .and_then(|x| x.as_str())
                                    .or_else(|| obj.get("symbol").and_then(|x| x.as_str()))
                                    .unwrap_or("");
                                if sym.is_empty() || normalize_sym(sym) == target_norm {
                                    return Some(isin_val.to_uppercase());
                                }
                            }
                        }

                        // Pipe-delimited "Value"
                        let value_str = obj
                            .get("Value")
                            .and_then(|x| x.as_str())
                            .or_else(|| obj.get("value").and_then(|x| x.as_str()))
                            .unwrap_or("");
                        if !value_str.is_empty() {
                            let parts: Vec<String> = value_str
                                .split('|')
                                .map(|p| p.trim().to_string())
                                .filter(|p| !p.is_empty())
                                .collect();
                            if let Some(isin) = pick_from_parts(&parts, target_norm) {
                                return Some(isin);
                            }
                        }

                        // Probe any string field if symbol matches
                        if let Some(sym) = obj
                            .get("Symbol")
                            .and_then(|x| x.as_str())
                            .or_else(|| obj.get("symbol").and_then(|x| x.as_str()))
                            && normalize_sym(sym) == target_norm
                        {
                            for (_k, v) in obj {
                                if let Some(s) = v.as_str()
                                    && looks_like_isin(s)
                                {
                                    return Some(s.to_uppercase());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    };

    // ---- Parse attempts ----
    let input_norm = normalize_sym(symbol);

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Some(hit) = extract_from_json_value(&val, &input_norm) {
            if debug {
                eprintln!(
                    "YF_DEBUG(isin): ISIN extracted from JSON structures: {hit}",
                );
            }
            return Ok(Some(hit));
        }
    } else if debug {
        eprintln!(
            "YF_DEBUG(isin): failed to parse JSON response for query '{query}'",
        );
    }

    if let Ok(raw_arr) = serde_json::from_str::<Vec<FlatSuggest>>(&body) {
        for r in &raw_arr {
            if let Some(isin) = r.isin.as_deref()
                && looks_like_isin(isin)
                && r.symbol.as_deref().map(normalize_sym) == Some(input_norm.clone())
            {
                return Ok(Some(isin.to_uppercase()));
            }
            if let Some(value) = r.value.as_deref() {
                let parts: Vec<String> = value
                    .split('|')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
                if let Some(isin) = pick_from_parts(&parts, &input_norm) {
                    return Ok(Some(isin));
                }
            }
        }
        // Fallback within the flat array: any ISIN-like token
        for r in &raw_arr {
            if let Some(isin) = r.isin.as_deref()
                && looks_like_isin(isin)
            {
                return Ok(Some(isin.to_uppercase()));
            }
            if let Some(value) = r.value.as_deref()
                && let Some(tok) = value
                    .split('|')
                    .map(str::trim)
                    .find(|tok| looks_like_isin(tok))
            {
                return Ok(Some((*tok).to_uppercase()));
            }
        }
    }

    // Raw-body scan fallback
    let mut token = String::new();
    for ch in body.chars() {
        if ch.is_ascii_alphanumeric() {
            token.push(ch);
            if token.len() > 12 {
                token.remove(0);
            }
            if token.len() == 12 && looks_like_isin(&token) {
                if debug {
                    eprintln!("YF_DEBUG(isin): Fallback raw scan found ISIN: {token}");
                }
                return Ok(Some(token.to_uppercase()));
            }
        } else {
            token.clear();
        }
    }

    if debug {
        eprintln!("YF_DEBUG(isin): No matching ISIN found in any response shape.");
    }
    Ok(None)
}
