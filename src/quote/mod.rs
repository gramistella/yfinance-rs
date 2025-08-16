use serde::Deserialize;
use url::Url;

use crate::core::Quote;
use crate::core::net;
use crate::core::{YfClient, YfError};

/* ---------------- Public API ---------------- */

/// Fetch a batch of quotes for multiple symbols using Yahoo's v7 endpoint.
/// Falls back to cookie+crumb auth automatically if the first call returns 401/403.
pub async fn quotes<I, S>(client: &mut YfClient, symbols: I) -> Result<Vec<Quote>, YfError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    QuotesBuilder::new(client)?.symbols(symbols).fetch().await
}

/// Builder for batch quote snapshots.
pub struct QuotesBuilder<'a> {
    client: &'a mut YfClient,
    quote_base: Url,
    symbols: Vec<String>,
}

impl<'a> QuotesBuilder<'a> {
    pub fn new(client: &'a mut YfClient) -> Result<Self, YfError> {
        Ok(Self {
            client,
            quote_base: Url::parse(DEFAULT_BASE_QUOTE_V7)?,
            symbols: Vec::new(),
        })
    }

    /// Override the v7 quote base URL (useful for tests).
    pub fn quote_base(mut self, base: Url) -> Self {
        self.quote_base = base;
        self
    }

    /// Set the symbols to query.
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(Into::into).collect();
        self
    }

    /// Add a single symbol.
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }

    /// Execute the request and return a vector of `Quote` (one per symbol found).
    pub async fn fetch(self) -> Result<Vec<Quote>, YfError> {
        if self.symbols.is_empty() {
            return Err(YfError::Data("quotes: at least one symbol required".into()));
        }

        // First try without crumb.
        let (body, _url, maybe_status) =
            fetch_v7_multi_raw(self.client, &self.quote_base, &self.symbols, None).await?;

        if let Some(code) = maybe_status
            && (code == 401 || code == 403)
        {
            return self.fetch_with_auth().await;
        }

        parse_v7_quotes(&body).map(|nodes| nodes.into_iter().map(map_v7_to_public).collect())
    }

    async fn fetch_with_auth(self) -> Result<Vec<Quote>, YfError> {
        self.client.ensure_credentials().await?;
        let crumb = self
            .client
            .crumb()
            .ok_or_else(|| YfError::Data("Crumb is not set".into()))?
            .to_string();

        let (body, _url, _status) =
            fetch_v7_multi_raw(self.client, &self.quote_base, &self.symbols, Some(&crumb)).await?;

        parse_v7_quotes(&body).map(|nodes| nodes.into_iter().map(map_v7_to_public).collect())
    }
}

/* ---------------- Internal helpers ---------------- */

const DEFAULT_BASE_QUOTE_V7: &str = "https://query1.finance.yahoo.com/v7/finance/quote";

async fn fetch_v7_multi_raw(
    client: &mut YfClient,
    base: &Url,
    symbols: &[String],
    crumb: Option<&str>,
) -> Result<(String, Url, Option<u16>), YfError> {
    let http = client.http().clone();

    let mut url = base.clone();
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("symbols", &symbols.join(","));
        if let Some(c) = crumb {
            qp.append_pair("crumb", c);
        }
    }

    // Fixture key policy:
    //  - single symbol => use the symbol
    //  - multi symbol => always "MULTI" so it lines up with tests/fixtures/quote_v7_MULTI.json
    let fixture_key_owned = if symbols.len() == 1 {
        symbols[0].clone()
    } else {
        "MULTI".to_string()
    };
    let fixture_key = fixture_key_owned.as_str();

    let resp = http
        .get(url.clone())
        .header("accept", "application/json")
        .send()
        .await?;

    let code = resp.status().as_u16();
    if !resp.status().is_success() {
        let body = net::get_text(resp, "quote_v7", fixture_key, "json").await?;
        return Ok((body, url, Some(code)));
    }

    let body = net::get_text(resp, "quote_v7", fixture_key, "json").await?;
    Ok((body, url, None))
}

fn parse_v7_quotes(body: &str) -> Result<Vec<V7QuoteNode>, YfError> {
    let env: V7Envelope =
        serde_json::from_str(body).map_err(|e| YfError::Data(format!("quote json parse: {e}")))?;
    let result = env
        .quote_response
        .and_then(|qr| qr.result)
        .unwrap_or_default();
    Ok(result)
}

fn map_v7_to_public(n: V7QuoteNode) -> Quote {
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

/* ---------------- Minimal serde for v7 quote ---------------- */

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

#[derive(Deserialize, Clone)]
struct V7QuoteNode {
    #[serde(default)]
    symbol: Option<String>,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketPreviousClose")]
    regular_market_previous_close: Option<f64>,
    currency: Option<String>,

    #[serde(rename = "fullExchangeName")]
    full_exchange_name: Option<String>,
    exchange: Option<String>,
    market: Option<String>,
    #[serde(rename = "marketCapFigureExchange")]
    market_cap_figure_exchange: Option<String>,

    #[serde(rename = "marketState")]
    market_state: Option<String>,
}
