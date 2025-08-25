use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::{
    select,
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::interval,
};
use tokio_tungstenite::{connect_async, connect_async_with_config, tungstenite::{handshake::client::{generate_key, Request}, protocol::{Message as WsMessage, WebSocketConfig}}};
use url::Url;

use crate::{YfClient, YfError};

mod wire_ws {
    include!(concat!(env!("OUT_DIR"), "/yaticker.rs"));
}

/// A real-time update for a financial instrument, typically received via a stream.
#[derive(Debug, Clone, PartialEq)]
pub struct QuoteUpdate {
    /// The ticker symbol for the instrument.
    pub symbol: String,
    /// The last traded price.
    pub last_price: Option<f64>,
    /// The previous day's closing price.
    ///
    /// Note: this is typically `None` when using WebSocket streaming.
    pub previous_close: Option<f64>,
    /// The currency of the instrument.
    pub currency: Option<String>,
    /// The timestamp of the update, as a Unix epoch timestamp.
    pub ts: i64,
}

/// Configuration for a polling-based quote stream.
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// The interval at which to poll for new quote data.
    pub interval: Duration,
    /// If `true`, only emit updates when the price has changed.
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

/// A handle to a running quote stream, used to stop it gracefully.
pub struct StreamHandle {
    join: JoinHandle<()>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl StreamHandle {
    /// Stops the stream and waits for the background task to complete.
    pub async fn stop(mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.join.await;
    }

    /// Aborts the background task immediately.
    pub fn abort(self) {
        self.join.abort();
    }
}

/// Defines the transport method for streaming quote data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamMethod {
    /// Attempt to use WebSockets, and fall back to polling if the connection fails. (Default)
    #[default]
    WebsocketWithFallback,
    /// Use WebSockets only. This is the preferred method for real-time data. The stream will fail if a WebSocket connection cannot be established.
    Websocket,
    /// Use polling over HTTP. This is a less efficient fallback option.
    Polling,
}

/// Builds and starts a real-time quote stream.
pub struct StreamBuilder {
    client: YfClient,
    symbols: Vec<String>,
    quote_base: Url,
    stream_url: Url,
    cfg: StreamConfig,
    method: StreamMethod,
}

impl StreamBuilder {
    /// Creates a new `StreamBuilder`.
    pub fn new(client: &YfClient) -> Result<Self, YfError> {
        Ok(Self {
            client: client.clone(),
            symbols: Vec::new(),
            quote_base: Url::parse(DEFAULT_BASE_QUOTE_V7)?,
            stream_url: Url::parse(DEFAULT_STREAM_URL)?,
            cfg: StreamConfig::default(),
            method: StreamMethod::default(),
        })
    }
    
    /// Sets the base URL for polling quote requests. (For testing purposes).
    pub fn quote_base(mut self, base: Url) -> Self {
        self.quote_base = base;
        self
    }
    
    /// Sets the URL for the WebSocket stream. (For testing purposes).
    pub fn stream_url(mut self, url: Url) -> Self {
        self.stream_url = url;
        self
    }

    /// Sets the symbols to stream.
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Adds a single symbol to the stream.
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }
    
    /// Sets the streaming transport method.
    pub fn method(mut self, method: StreamMethod) -> Self {
        self.method = method;
        self
    }

    /// Sets the polling interval. (Only used for `Polling` and `WebsocketWithFallback` methods).
    pub fn interval(mut self, dur: Duration) -> Self {
        self.cfg.interval = dur;
        self
    }

    /// If `true`, only emit updates when the price changes. (Only used for `Polling` method).
    pub fn diff_only(mut self, yes: bool) -> Self {
        self.cfg.diff_only = yes;
        self
    }

    /// Starts the stream, returning a handle to control it and a receiver for quote updates.
    pub fn start(self) -> Result<(StreamHandle, mpsc::Receiver<QuoteUpdate>), YfError> {
        if self.symbols.is_empty() {
            return Err(YfError::Data("stream: at least one symbol required".into()));
        }

        let (tx, rx) = mpsc::channel::<QuoteUpdate>(1024);
        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        let join = tokio::spawn(async move {
            let mut client = self.client;
            let symbols = self.symbols;
            let cfg = self.cfg;
            let quote_base = self.quote_base;
            let stream_url = self.stream_url;
            let mut stop_rx = stop_rx;

            match self.method {
                StreamMethod::Websocket => {
                    if let Err(e) =
                        run_websocket_stream(&mut client, symbols, stream_url, tx, &mut stop_rx)
                            .await
                    {
                        if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                            eprintln!("YF_DEBUG(stream): websocket stream failed: {e}");
                        }
                    }
                }
                StreamMethod::WebsocketWithFallback => {
                    if let Err(e) =
                        run_websocket_stream(
                            &mut client,
                            symbols.clone(),
                            stream_url,
                            tx.clone(),
                            &mut stop_rx,
                        )
                        .await
                    {
                        if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                            eprintln!(
                                "YF_DEBUG(stream): websocket failed ({e}), falling back to polling."
                            );
                        }
                        run_polling_stream(client, symbols, quote_base, cfg, tx, &mut stop_rx)
                            .await;
                    }
                }
                StreamMethod::Polling => {
                    run_polling_stream(client, symbols, quote_base, cfg, tx, &mut stop_rx).await;
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

const DEFAULT_BASE_QUOTE_V7: &str = "https://query1.finance.yahoo.com/v7/finance/quote";
const DEFAULT_STREAM_URL: &str = "wss://streamer.finance.yahoo.com/?version=2";

#[derive(Serialize)]
struct WsSubscribe<'a> {
    subscribe: &'a [String],
}

async fn run_websocket_stream(
    client: &mut YfClient,
    symbols: Vec<String>,
    stream_url: Url,
    tx: mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut oneshot::Receiver<()>,
) -> Result<(), YfError> {
    let host = stream_url
        .host_str()
        .ok_or_else(|| YfError::Data("URL has no host".into()))?;

    let request = Request::builder()
        .uri(stream_url.as_str())
        .header("Host", host)
        .header("Origin", "https://finance.yahoo.com")
        .header("User-Agent", client.user_agent())
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", generate_key())
        .header("Sec-WebSocket-Version", "13")
        .body(())
        .map_err(|e| YfError::Data(format!("Failed to build websocket request: {e}")))?;

    let (ws_stream, _) = connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();

    let sub_msg = serde_json::to_string(&WsSubscribe { subscribe: &symbols })
        .map_err(|e| YfError::Data(format!("ws subscribe serialize: {e}")))?;
    write.send(WsMessage::Text(sub_msg)).await?;

    #[cfg(feature = "test-mode")]
    let mut recorded = false;

    loop {
        select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        #[cfg(feature = "test-mode")]
                        {
                            if !recorded && std::env::var("YF_RECORD").ok().as_deref() == Some("1") {
                                if let Err(e) = crate::core::fixtures::record_fixture("stream_ws", "MULTI", "b64", &text) {
                                    eprintln!("YF_RECORD: failed to write stream fixture: {e}");
                                }
                                recorded = true;
                            }
                        }

                        match decode_and_map_message(&text) {
                            Ok(update) => {
                                let _ = tx.send(update).await;
                            },
                            Err(e) => {
                                if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                                    eprintln!("YF_DEBUG(stream): ws text decode error: {e}");
                                }
                                // Non-price frames (acks/heartbeats) may lack "message"; ignore.
                            }
                        }
                    }
                    Some(Ok(WsMessage::Binary(bin))) => {
                        // Try to interpret as UTF-8 JSON-wrapped base64 first
                        let mut handled = false;
                        if let Ok(as_text) = std::str::from_utf8(&bin) {
                            if let Ok(update) = decode_and_map_message(as_text) {
                                let _ = tx.send(update).await;
                                handled = true;
                            }
                        }
                        // If not handled, treat as raw protobuf bytes
                        if !handled {
                            match wire_ws::PricingData::decode(&*bin) {
                                Ok(ticker) => {
                                    let update = QuoteUpdate {
                                        symbol: ticker.id,
                                        last_price: Some(ticker.price as f64),
                                        previous_close: Some(ticker.previous_close as f64),
                                        currency: Some(ticker.currency),
                                        ts: ticker.time,
                                    };
                                    let _ = tx.send(update).await;
                                }
                                Err(e) => {
                                    if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                                        eprintln!("YF_DEBUG(stream): ws binary decode error: {e}");
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(WsMessage::Ping(_))) => { /* tungstenite handles pongs */ }
                    Some(Ok(WsMessage::Pong(_))) => { /* ignore */ }
                    Some(Ok(WsMessage::Close(_))) => { break; }
                    Some(Ok(_)) => { /* catch-all for variants like Frame(_) */ }
                    Some(Err(e)) => return Err(e.into()),
                    None => break,
                }
            },
            _ = &mut *stop_rx => {
                break;
            }
        }
    }
    Ok(())
}

/// Decodes a single base64-encoded protobuf message from the Yahoo Finance WebSocket stream.
#[doc(hidden)]
pub fn decode_and_map_message(text: &str) -> Result<QuoteUpdate, YfError> {
    // Support both:
    //   1) Raw base64 string
    //   2) JSON wrapper: {"message":"<base64...>"}  (Yahoo's current format)
    let s = text.trim();

    // Use Cow to avoid borrowing from a temporary JSON value
    let b64_cow: std::borrow::Cow<str> = if s.starts_with('{') {
        match serde_json::from_str::<serde_json::Value>(s) {
            Ok(v) => {
                let msg = v
                    .get("message")
                    .and_then(|m| m.as_str())
                    .ok_or_else(|| YfError::Data("ws json message missing 'message' field".into()))?;
                std::borrow::Cow::Owned(msg.to_string())
            }
            // If it's not valid JSON, treat the whole thing as raw base64
            Err(_) => std::borrow::Cow::Borrowed(s),
        }
    } else {
        std::borrow::Cow::Borrowed(s)
    };

    let decoded = general_purpose::STANDARD
        .decode(b64_cow.as_ref())
        .map_err(|e| YfError::Data(format!("base64 decode error: {e}")))?;
    let ticker = wire_ws::PricingData::decode(&*decoded)?;
    Ok(QuoteUpdate {
        symbol: ticker.id,
        last_price: Some(ticker.price as f64),
        previous_close: Some(ticker.previous_close as f64),
        currency: Some(ticker.currency),
        ts: ticker.time,
    })
}

async fn run_polling_stream(
    mut client: YfClient,
    symbols: Vec<String>,
    base: Url,
    cfg: StreamConfig,
    tx: mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut oneshot::Receiver<()>,
) {
    let mut ticker = interval(cfg.interval);
    let mut last_price: HashMap<String, Option<f64>> = HashMap::new();
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
                                    continue;
                                }
                            }
                            if tx.send(QuoteUpdate {
                                symbol: q.symbol,
                                last_price: lp,
                                previous_close: q.regular_market_previous_close,
                                currency: q.currency,
                                ts,
                            }).await.is_err() {
                                break;
                            }
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
            _ = &mut *stop_rx => {
                break;
            }
        }
    }
}

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
        let joined = symbols.join(",");
        qp.append_pair("symbols", &joined);
    }

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
    let body = crate::core::net::get_text(resp, "quote_v7", fixture_key, "json").await?;
    let env: V7Envelope =
        serde_json::from_str(&body).map_err(|e| YfError::Data(format!("quote json parse: {e}")))?;

    let result = env
        .quote_response
        .and_then(|qr| qr.result)
        .unwrap_or_default();

    Ok(result)
}

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