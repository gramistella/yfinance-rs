use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use serde::Serialize;
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
    core::conversions::{f64_to_money_with_currency_str, i64_to_datetime},
};
use paft::market::quote::QuoteUpdate;

mod wire_ws {
    include!(concat!(env!("OUT_DIR"), "/yaticker.rs"));
}

// Use paft's QuoteUpdate which carries Money and DateTime<Utc>
// pub use paft::market::quote::QuoteUpdate; (imported above)

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
    /// Attempt to use `WebSockets`, and fall back to polling if the connection fails. (Default)
    #[default]
    WebsocketWithFallback,
    /// Use `WebSockets` only. This is the preferred method for real-time data. The stream will fail if a WebSocket connection cannot be established.
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
    #[must_use]
    pub fn new(client: &YfClient) -> Self {
        Self {
            client: client.clone(),
            symbols: Vec::new(),
            cfg: StreamConfig::default(),
            method: StreamMethod::default(),
            cache_mode: CacheMode::Use,
            retry_override: None,
        }
    }

    /// Sets the cache mode for this specific API call (only affects polling mode).
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call (only affects polling mode).
    #[must_use]
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.retry_override = cfg;
        self
    }

    /// Sets the symbols to stream.
    #[must_use]
    pub fn symbols<I, S>(mut self, syms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = syms.into_iter().map(std::convert::Into::into).collect();
        self
    }

    /// Adds a single symbol to the stream.
    #[must_use]
    pub fn add_symbol(mut self, sym: impl Into<String>) -> Self {
        self.symbols.push(sym.into());
        self
    }

    /// Sets the streaming transport method.
    #[must_use]
    pub const fn method(mut self, method: StreamMethod) -> Self {
        self.method = method;
        self
    }

    /// Sets the polling interval. (Only used for `Polling` and `WebsocketWithFallback` methods).
    #[must_use]
    pub const fn interval(mut self, dur: Duration) -> Self {
        self.cfg.interval = dur;
        self
    }

    /// If `true`, only emit updates when the price changes. (Only used for `Polling` method).
    #[must_use]
    pub const fn diff_only(mut self, yes: bool) -> Self {
        self.cfg.diff_only = yes;
        self
    }

    /// Starts the stream, returning a handle to control it and a channel receiver for quote updates.
    ///
    /// # Errors
    ///
    /// This method will return an error if no symbols have been added to the builder.
    pub fn start(
        self,
    ) -> Result<(StreamHandle, tokio::sync::mpsc::Receiver<QuoteUpdate>), crate::core::YfError>
    {
        if self.symbols.is_empty() {
            return Err(crate::core::YfError::InvalidParams(
                "symbols list cannot be empty".into(),
            ));
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<QuoteUpdate>(1024);
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

        let join = tokio::spawn({
            let client = self.client;
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
                            run_websocket_stream(&client, symbols, tx, &mut stop_rx).await
                            && std::env::var("YF_DEBUG").ok().as_deref() == Some("1")
                        {
                            eprintln!("YF_DEBUG(stream): websocket stream failed: {e}");
                        }
                    }
                    StreamMethod::WebsocketWithFallback => {
                        if let Err(e) =
                            run_websocket_stream(&client, symbols.clone(), tx.clone(), &mut stop_rx)
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

#[allow(clippy::too_many_lines)]
async fn run_websocket_stream(
    client: &YfClient,
    symbols: Vec<String>,
    tx: mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut oneshot::Receiver<()>,
) -> Result<(), YfError> {
    let base = client.base_stream();
    let host = base
        .host_str()
        .ok_or_else(|| YfError::InvalidParams("URL has no host".into()))?;

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
        .map_err(|e| YfError::InvalidParams(format!("Failed to build websocket request: {e}")))?;

    let (ws_stream, _) = connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();

    let sub_msg = serde_json::to_string(&WsSubscribe {
        subscribe: &symbols,
    })
    .map_err(YfError::Json)?;
    write.send(WsMessage::Text(sub_msg.into())).await?;

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
                                if tx.send(update).await.is_err() {
                                    break; // Receiver was dropped, exit loop
                                }
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
                        let handled = if let Ok(as_text) = std::str::from_utf8(&bin)
                            && let Ok(update) = decode_and_map_message(as_text) {
                                if tx.send(update).await.is_err() {
                                    break; // Receiver was dropped
                                }
                                true
                            } else { false };
                        // If not handled, treat as raw protobuf bytes
                        if !handled {
                            match wire_ws::PricingData::decode(&*bin) {
                                Ok(ticker) => {
                                    let currency_str = Some(ticker.currency.as_str());
                                    let Ok(symbol) = paft::domain::Symbol::new(&ticker.id) else {
                                        if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                                            eprintln!("YF_DEBUG(stream): skipping ws update with invalid symbol: {}", ticker.id);
                                        }
                                        continue;
                                    };
                                    let update = QuoteUpdate {
                                        symbol,
                                        price: Some(f64_to_money_with_currency_str(f64::from(ticker.price), currency_str)),
                                        previous_close: Some(f64_to_money_with_currency_str(f64::from(ticker.previous_close), currency_str)),
                                        ts: i64_to_datetime(ticker.time),
                                    };
                                    if tx.send(update).await.is_err() {
                                        break; // Receiver was dropped
                                    }
                                }
                                Err(e) => {
                                    if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                                        eprintln!("YF_DEBUG(stream): ws binary decode error: {e}");
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(WsMessage::Ping(_) | WsMessage::Pong(_) | _)) => { /* catch-all for variants like Frame(_) */ }
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
                    YfError::MissingData("ws json message missing 'message' field".into())
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
        .map_err(YfError::Base64)?;
    let ticker = wire_ws::PricingData::decode(&*decoded)?;
    let currency_str = Some(ticker.currency.as_str());
    let symbol = paft::domain::Symbol::new(&ticker.id)
        .map_err(|_| YfError::InvalidParams(format!("ws symbol invalid: {}", ticker.id)))?;
    Ok(QuoteUpdate {
        symbol,
        price: Some(f64_to_money_with_currency_str(
            f64::from(ticker.price),
            currency_str,
        )),
        previous_close: Some(f64_to_money_with_currency_str(
            f64::from(ticker.previous_close),
            currency_str,
        )),
        ts: i64_to_datetime(ticker.time),
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
                if tx.is_closed() { break; }
                let ts: DateTime<Utc> = chrono::Utc::now();
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
                            let currency_str = q.currency.as_deref();
                            let sym_s = q.symbol.clone().unwrap_or_default();
                            let Ok(symbol) = paft::domain::Symbol::new(&sym_s) else { continue };
                            if tx.send(QuoteUpdate {
                                symbol,
                                price: lp.map(|v| f64_to_money_with_currency_str(v, currency_str)),
                                previous_close: q.regular_market_previous_close.map(|v| f64_to_money_with_currency_str(v, currency_str)),
                                ts,
                            }).await.is_err() {
                                // Break outer loop if receiver is dropped
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
