use base64::{Engine as _, engine::general_purpose};
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use serde::{Serialize};
use std::time::Duration;
use tokio::{
    select,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        handshake::client::{Request, generate_key},
        protocol::Message as WsMessage,
    },
};

use crate::{
    YfClient, YfError,
    core::client::{CacheMode, RetryConfig},
};

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
    cfg: StreamConfig,
    method: StreamMethod,
    cache_mode: CacheMode,
    retry_override: Option<RetryConfig>,
}

impl StreamBuilder {
    /// Creates a new `StreamBuilder`.
    pub fn new(client: &YfClient) -> Result<Self, YfError> {
        Ok(Self {
            client: client.clone(),
            symbols: Vec::new(),
            cfg: StreamConfig::default(),
            method: StreamMethod::default(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        })
    }

    /// Sets the cache mode for this specific API call (only affects polling mode).
    pub fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call (only affects polling mode).
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
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

    /// Starts the stream, returning a handle to control it and a channel receiver for quote updates.
    pub fn start(
        self,
    ) -> Result<(StreamHandle, tokio::sync::mpsc::Receiver<QuoteUpdate>), crate::core::YfError>
    {
        if self.symbols.is_empty() {
            return Err(crate::core::YfError::Data(
                "stream: at least one symbol required".into(),
            ));
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<QuoteUpdate>(1024);
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

        let join = tokio::spawn({
            let mut client = self.client.clone();
            let symbols = self.symbols.clone();
            let cfg = self.cfg.clone();

            let mut stop_rx = stop_rx;

            // NEW:
            let cache_mode = self.cache_mode;
            let retry_override = self.retry_override.clone();

            async move {
                match self.method {
                    StreamMethod::Websocket => {
                        if let Err(e) =
                            run_websocket_stream(&mut client, symbols, tx, &mut stop_rx).await
                            && std::env::var("YF_DEBUG").ok().as_deref() == Some("1")
                        {
                            eprintln!("YF_DEBUG(stream): websocket stream failed: {e}");
                        }
                    }
                    StreamMethod::WebsocketWithFallback => {
                        if let Err(e) = run_websocket_stream(
                            &mut client,
                            symbols.clone(),
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
                            run_polling_stream(
                                client,
                                symbols,
                                cfg,
                                tx,
                                &mut stop_rx,
                                cache_mode,
                                retry_override.as_ref(),
                            )
                            .await;
                        }
                    }
                    StreamMethod::Polling => {
                        run_polling_stream(
                            client,
                            symbols,
                            cfg,
                            tx,
                            &mut stop_rx,
                            cache_mode,
                            retry_override.as_ref(),
                        )
                        .await;
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

#[derive(Serialize)]
struct WsSubscribe<'a> {
    subscribe: &'a [String],
}

async fn run_websocket_stream(
    client: &mut YfClient,
    symbols: Vec<String>,
    tx: mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut oneshot::Receiver<()>,
) -> Result<(), YfError> {
    let base = client.base_stream();
    let host = base
        .host_str()
        .ok_or_else(|| YfError::Data("URL has no host".into()))?;

    let request = Request::builder()
        .uri(base.as_str())
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

    let sub_msg = serde_json::to_string(&WsSubscribe {
        subscribe: &symbols,
    })
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
                        if let Ok(as_text) = std::str::from_utf8(&bin)
                            && let Ok(update) = decode_and_map_message(as_text) {
                                let _ = tx.send(update).await;
                                handled = true;
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
                let msg = v.get("message").and_then(|m| m.as_str()).ok_or_else(|| {
                    YfError::Data("ws json message missing 'message' field".into())
                })?;
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

#[allow(clippy::too_many_arguments)]
async fn run_polling_stream(
    client: crate::core::YfClient,
    symbols: Vec<String>,
    cfg: StreamConfig,
    tx: tokio::sync::mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut tokio::sync::oneshot::Receiver<()>,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) {
    let mut ticker = tokio::time::interval(cfg.interval);
    let mut last_price: std::collections::HashMap<String, Option<f64>> =
        std::collections::HashMap::new();
    
    let symbol_slices: Vec<&str> = symbols.iter().map(AsRef::as_ref).collect();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let ts = chrono::Utc::now().timestamp();
                match crate::core::quotes::fetch_v7_quotes(&client, &symbol_slices, cache_mode, retry_override).await {
                    Ok(quotes) => {
                        for q in quotes {
                            let lp = q.regular_market_price.or(q.regular_market_previous_close);
                            if cfg.diff_only {
                                let symbol = q.symbol.clone().unwrap_or_default();
                                let prev = last_price.insert(symbol, lp);
                                if prev == Some(lp) {
                                    continue;
                                }
                            }
                            if tx.send(QuoteUpdate {
                                symbol: q.symbol.unwrap_or_default(),
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
                if tx.is_closed() { break; }
            }
            _ = &mut *stop_rx => { break; }
        }
    }
}
