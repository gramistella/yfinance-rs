use crate::{history::HistoryBuilder, internal::net, YfClient, YfError};
use serde::Deserialize;
use url::Url;

/// Default Yahoo Finance v7 quote endpoint.
/// Example: https://query1.finance.yahoo.com/v7/finance/quote?symbols=AAPL,MSFT
const DEFAULT_BASE_QUOTE_V7: &str = "https://query1.finance.yahoo.com/v7/finance/quote";

/// High-level façade similar to yfinance's `Ticker`.
/// Keeps a reference to your client and a symbol, and exposes convenient methods.
pub struct Ticker<'a> {
    client: &'a mut YfClient,
    symbol: String,
    quote_base: Url,
}

impl<'a> Ticker<'a> {
    /// Construct with the default Yahoo v7 quote endpoint.
    pub fn new(client: &'a mut YfClient, symbol: impl Into<String>) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base: Url::parse(DEFAULT_BASE_QUOTE_V7)?,
        })
    }

    /// Construct with a custom base URL for v7 quotes (useful for tests/mocks).
    ///
    /// Example base: `{mock_server}/v7/finance/quote`
    pub fn with_quote_base(
        client: &'a mut YfClient,
        symbol: impl Into<String>,
        base: Url,
    ) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base: base,
        })
    }

    /// Minimal convenience to start building a history request for this ticker.
    pub fn history_builder(&self) -> HistoryBuilder<'_> {
        // reborrow &mut YfClient as &YfClient for the builder
        HistoryBuilder::new(&*self.client, &self.symbol)
    }

    /// Fetch a lightweight "quote snapshot" for this ticker from the v7 quote endpoint.
    ///
    /// First attempt: no crumb/cookie preflight (works for most cases and for synthetic tests).
    /// If we get a 401/403, we fetch credentials (cookie + crumb) and retry once with `crumb=...`.
    pub async fn quote(&mut self) -> Result<Quote, YfError> {
        // Clone the reqwest client up front so we don't hold an immutable borrow of `self.client`
        // across any await where we later need `&mut self.client` for ensure_credentials().
        let http = self.client.http().clone();

        // attempt #1 (no crumb)
        let mut url = self.quote_base.clone();
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("symbols", &self.symbol);
        }

        let mut resp = http
            .get(url.clone())
            .header("accept", "application/json")
            .send()
            .await?;

        if resp.status().is_success() {
            return self.parse_quote(resp).await;
        }

        let code = resp.status().as_u16();
        if code != 401 && code != 403 {
            return Err(YfError::Status {
                status: code,
                url: url.to_string(),
            });
        }

        // attempt #2: ensure cookie + crumb, then include crumb in query and retry
        self.client.ensure_credentials().await?;
        let crumb = match self.client.crumb() {
            Some(c) => c.to_string(),
            None => {
                return Err(YfError::Status {
                    status: code,
                    url: url.to_string(),
                })
            }
        };

        let mut url2 = self.quote_base.clone();
        {
            let mut qp = url2.query_pairs_mut();
            qp.append_pair("symbols", &self.symbol);
            qp.append_pair("crumb", &crumb);
        }

        resp = http
            .get(url2.clone())
            .header("accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(YfError::Status {
                status: resp.status().as_u16(),
                url: url2.to_string(),
            });
        }

        self.parse_quote(resp).await
    }

    async fn parse_quote(&self, resp: reqwest::Response) -> Result<Quote, YfError> {
        let body = net::get_text(resp, "quote_v7", &self.symbol, "json").await?;
        let env: V7Envelope =
            serde_json::from_str(&body).map_err(|e| YfError::Data(format!("quote json parse: {e}")))?;

        let result = env
            .quote_response
            .and_then(|qr| qr.result)
            .and_then(|mut v| v.pop())
            .ok_or_else(|| YfError::Data("empty quote result".into()))?;

        Ok(Quote {
            symbol: result.symbol.unwrap_or_else(|| self.symbol.clone()),
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

    /// A tiny derived struct similar to yfinance's `fast_info`.
    ///
    /// `last_price` is taken from `regularMarketPrice`. If that is missing, we fall back to
    /// `regularMarketPreviousClose` to avoid hard errors; if both are missing, it's an error.
    pub async fn fast_info(&mut self) -> Result<FastInfo, YfError> {
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

    /// yfinance-style convenience: fetch history with optional range/interval,
    /// defaulting to auto-adjusted daily bars and no pre/post.
    pub async fn history(
        &self,
        range: Option<crate::Range>,
        interval: Option<crate::Interval>,
        prepost: bool,
    ) -> Result<Vec<crate::Candle>, YfError> {
        let mut hb = self.history_builder();
        if let Some(r) = range {
            hb = hb.range(r);
        }
        if let Some(i) = interval {
            hb = hb.interval(i);
        }
        hb = hb.auto_adjust(true).prepost(prepost).actions(true);
        hb.fetch().await
    }

    /// Convenience: return all corporate actions (splits/dividends) over a range.
    /// If range is None, defaults to `Range::Max`.
    pub async fn actions(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<crate::Action>, YfError> {
        let mut hb = self.history_builder();
        hb = hb.range(range.unwrap_or(crate::Range::Max));
        let resp = hb.auto_adjust(true).actions(true).fetch_full().await?;
        Ok(resp.actions)
    }

    /// Convenience: only dividends (ts, amount)
    pub async fn dividends(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<(i64, f64)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Dividend { ts, amount } => Some((ts, amount)),
                _ => None,
            })
            .collect())
    }

    /// Convenience: only splits (ts, numerator, denominator)
    pub async fn splits(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<(i64, u32, u32)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Split { ts, numerator, denominator } => Some((ts, numerator, denominator)),
                _ => None,
            })
            .collect())
    }

    /// Convenience: read minimal metadata (timezone/gmtoffset) from the chart meta.
    pub async fn get_history_metadata(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Option<crate::HistoryMeta>, YfError> {
        let mut hb = self.history_builder();
        if let Some(r) = range {
            hb = hb.range(r);
        }
        let resp = hb.fetch_full().await?;
        Ok(resp.meta)
    }
}

/* ---------------- Public models ---------------- */

#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub symbol: String,
    pub regular_market_price: Option<f64>,
    pub regular_market_previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FastInfo {
    pub symbol: String,
    pub last_price: f64,
    pub previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,
}

/* ---------------- Minimal serde mapping for v7 quote ---------------- */

#[derive(Deserialize)]
struct V7Envelope {
    #[serde(rename = "quoteResponse")]
    quote_response: Option<V7QuoteResponse>,
}

#[derive(Deserialize)]
struct V7QuoteResponse {
    result: Option<Vec<V7QuoteNode>>,
    #[allow(dead_code)]
    error: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct V7QuoteNode {
    #[serde(default)]
    symbol: Option<String>,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketPreviousClose")]
    regular_market_previous_close: Option<f64>,
    currency: Option<String>,

    // exchange-ish fields (not all always present)
    #[serde(rename = "fullExchangeName")]
    full_exchange_name: Option<String>,
    exchange: Option<String>,
    market: Option<String>,
    #[serde(rename = "marketCapFigureExchange")]
    market_cap_figure_exchange: Option<String>,

    #[serde(rename = "marketState")]
    market_state: Option<String>,
}
