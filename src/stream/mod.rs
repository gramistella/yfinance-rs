use chrono::Utc;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use tokio::{
    select,
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::interval,
};
use url::Url;

use crate::{YfClient, YfError};

/* ---------------- Public API ---------------- */

/// One streaming quote update per symbol (and poll tick).
#[derive(Debug, Clone, PartialEq)]
pub struct QuoteUpdate {
    pub symbol: String,
    pub last_price: Option<f64>,
    pub previous_close: Option<f64>,
    pub currency: Option<String>,
    /// Unix seconds (UTC) when this update was produced client-side.
    pub ts: i64,
}

/// Configure polling behavior.
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Poll cadence. Default: 1s.
    pub interval: Duration,
    /// Only emit an update when the symbol’s `last_price` changed since previous tick.
    /// Default: true.
    pub diff_only: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(1),
            diff_only: true,
        }
    }
}

/// A handle for a running stream task.
pub struct StreamHandle {
    join: JoinHandle<()>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl StreamHandle {
    /// Politely ask the stream to stop and wait for it to finish.
    pub async fn stop(mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.join.await;
    }

    /// Immediately abort the background task (no more messages will be sent).
    pub fn abort(self) {
        self.join.abort();
    }
}

/// Builder to start a streaming task for one or more symbols.
pub struct StreamBuilder {
    client: YfClient,
    symbols: Vec<String>,
    quote_base: Url,
    cfg: StreamConfig,
}

impl StreamBuilder {
    /// Start from an existing client (cloned internally).
    pub fn new(client: &YfClient) -> Result<Self, YfError> {
        Ok(Self {
            client: client.clone(),
            symbols: Vec::new(),
            quote_base: Url::parse(DEFAULT_BASE_QUOTE_V7)?,
            cfg: StreamConfig::default(),
        })
    }

    /// Use a non-default quote base (handy for tests/mocks).
    pub fn quote_base(mut self, base: Url) -> Self {
        self.quote_base = base;
        self
    }

    /// Stream these symbols (replaces).
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Add a single symbol.
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }

    /// Poll interval.
    pub fn interval(mut self, dur: Duration) -> Self {
        self.cfg.interval = dur;
        self
    }

    /// Emit only on price changes (default true).
    pub fn diff_only(mut self, yes: bool) -> Self {
        self.cfg.diff_only = yes;
        self
    }

    /// Start the stream. Returns a handle and a receiver of updates.
    ///
    /// Drop the receiver to naturally stop when the sender backpressure closes,
    /// or call `handle.stop().await` / `handle.abort()`.
    pub fn start(self) -> Result<(StreamHandle, mpsc::Receiver<QuoteUpdate>), YfError> {
        if self.symbols.is_empty() {
            return Err(YfError::Data("stream: at least one symbol required".into()));
        }

        // channel for updates
        let (tx, rx) = mpsc::channel::<QuoteUpdate>(1024);
        // stop signal
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();

        // move into task
        let mut client = self.client;
        let symbols = self.symbols;
        let base = self.quote_base;
        let cfg = self.cfg;

        let join = tokio::spawn(async move {
            let mut ticker = interval(cfg.interval);
            // last seen price per symbol (for diff_only)
            let mut last_price: HashMap<String, Option<f64>> = HashMap::new();

            // Precompute a stable fixture key that matches how we record caches in live runs.
            // Using the joined symbol list ensures:
            //  - single-symbol streams reuse the same file as regular quote calls (e.g. "AAPL")
            //  - multi-symbol streams create/read "AAPL,MSFT" etc.
            let fixture_key = symbols.join(",");

            loop {
                select! {
                    _ = ticker.tick() => {
                        let ts = Utc::now().timestamp();

                        match fetch_quotes_multi(&mut client, &base, &symbols, &fixture_key).await {
                            Ok(quotes) => {
                                for q in quotes {
                                    let lp = q.regular_market_price.or(q.regular_market_previous_close);
                                    if cfg.diff_only {
                                        let prev = last_price.insert(q.symbol.clone(), lp);
                                        if prev == Some(lp) {
                                            continue; // unchanged; skip
                                        }
                                    }
                                    let _ = tx.send(QuoteUpdate {
                                        symbol: q.symbol,
                                        last_price: lp,
                                        previous_close: q.regular_market_previous_close,
                                        currency: q.currency,
                                        ts,
                                    }).await;
                                }
                            }
                            Err(e) => {
                                if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                                    eprintln!("YF_DEBUG(stream): fetch error: {e}");
                                }
                            }
                        }

                        if tx.is_closed() {
                            break;
                        }
                    }
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });

        Ok((
            StreamHandle {
                join,
                stop_tx: Some(stop_tx),
            },
            rx,
        ))
    }
}

/* ---------------- Internal: multi-quote fetch (v7) ---------------- */

const DEFAULT_BASE_QUOTE_V7: &str = "https://query1.finance.yahoo.com/v7/finance/quote";

async fn fetch_quotes_multi(
    client: &mut YfClient,
    base: &Url,
    symbols: &[String],
    fixture_key: &str,
) -> Result<Vec<V7QuoteNode>, YfError> {
    let http = client.http().clone();

    let mut url = base.clone();
    {
        let mut qp = url.query_pairs_mut();
        // join symbols with commas
        let joined = symbols.join(",");
        qp.append_pair("symbols", &joined);
    }

    // First attempt without crumb (often fine)
    let mut resp = http
        .get(url.clone())
        .header("accept", "application/json")
        .send()
        .await?;

    if resp.status().is_success() {
        return parse_v7_multi(resp, fixture_key).await;
    }

    let code = resp.status().as_u16();
    if code != 401 && code != 403 {
        return Err(crate::YfError::Status {
            status: code,
            url: url.to_string(),
        });
    }

    // Retry with crumb
    client.ensure_credentials().await?;
    let crumb = client
        .crumb()
        .ok_or_else(|| crate::YfError::Status {
            status: code,
            url: url.to_string(),
        })?
        .to_string();

    let mut url2 = base.clone();
    {
        let mut qp = url2.query_pairs_mut();
        let joined = symbols.join(",");
        qp.append_pair("symbols", &joined);
        qp.append_pair("crumb", &crumb);
    }

    resp = http
        .get(url2.clone())
        .header("accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(crate::YfError::Status {
            status: resp.status().as_u16(),
            url: url2.to_string(),
        });
    }

    parse_v7_multi(resp, fixture_key).await
}

async fn parse_v7_multi(
    resp: reqwest::Response,
    fixture_key: &str,
) -> Result<Vec<V7QuoteNode>, YfError> {
    // IMPORTANT: the fixture key *must* map to your live recording.
    // For single-symbol streams this is exactly the same as Ticker::quote ("AAPL"),
    // so the stream happily reuses quote_v7/AAPL.json recorded during live runs.
    let body = crate::core::net::get_text(resp, "quote_v7", fixture_key, "json").await?;
    let env: V7Envelope =
        serde_json::from_str(&body).map_err(|e| YfError::Data(format!("quote json parse: {e}")))?;

    let result = env
        .quote_response
        .and_then(|qr| qr.result)
        .unwrap_or_default();

    Ok(result)
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
    symbol: String,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketPreviousClose")]
    regular_market_previous_close: Option<f64>,
    currency: Option<String>,
}
