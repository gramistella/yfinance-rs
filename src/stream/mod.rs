use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use reqwest::{
    StatusCode, Version,
    header::{
        CONNECTION, ORIGIN, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION,
        UPGRADE, USER_AGENT,
    },
};
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, time::Duration};
use tokio::{
    select,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Error as WsError,
        handshake::{client::generate_key, derive_accept_key},
        protocol::{Message as WsMessage, Role},
    },
};
use url::Url;

use crate::{
    YfClient, YfError,
    core::CallOptions,
    core::client::{CacheMode, RetryConfig, normalize_symbols},
    core::conversions::{decimal_from_f32, quantity_from_i64, quantity_from_u64},
    core::currency_resolver::ResolvedCurrencyUnit,
    core::yahoo_vocab::{parse_yahoo_quote_type, yahoo_exchange_to_listing_currency},
    core::{error::RedactedHttpError, redaction::RedactedUrl},
};
use paft::Decimal;
use paft::domain::{AssetKind, Instrument};
use paft::market::quote::QuoteUpdate;
use paft::money::{PriceAmount, QuantityAmount};

const UNTYPED_STREAM_ASSET_KIND: &str = "YAHOO_STREAM_UNTYPED";
const DEFAULT_WEBSOCKET_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_WEBSOCKET_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_WEBSOCKET_RECONNECT_BACKOFF: Duration = Duration::from_secs(30);

// Yahoo Finance websocket wire types (generated from `yaticker.proto`).
mod wire_ws {
    include!("yaticker.rs");
}

fn untyped_stream_asset_kind() -> AssetKind {
    AssetKind::other(UNTYPED_STREAM_ASSET_KIND).expect("valid stream fallback asset kind")
}

// Streaming quotes
//
// Volume semantics:
// - Yahoo sends cumulative volume (`day_volume` / `regularMarketVolume`).
//   `QuoteUpdate::volume` exposes that latest cumulative value directly.
// - This crate deliberately does not infer per-update deltas, session boundaries,
//   resets, or provider-side adjustments. Callers that need those semantics can
//   derive them from successive cumulative values with their own boundary policy.
// - When Yahoo omits instrument type data, or sends an unknown stream quote type, and no
//   typed instrument is cached, the emitted update uses
//   `AssetKind::Other("YAHOO_STREAM_UNTYPED")`. That fallback is deliberately not cached,
//   so later typed quote data can replace it.
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
    /// Attempt to use `WebSockets`, using polling between reconnect attempts. (Default)
    #[default]
    WebsocketWithFallback,
    /// Use `WebSockets` only.
    ///
    /// This is the preferred method for real-time data. `StreamBuilder::start` will fail if the
    /// initial WebSocket connection or subscription cannot be established.
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
    ws_connect_timeout: Duration,
    ws_idle_timeout: Duration,
    options: CallOptions,
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
            ws_connect_timeout: DEFAULT_WEBSOCKET_CONNECT_TIMEOUT,
            ws_idle_timeout: DEFAULT_WEBSOCKET_IDLE_TIMEOUT,
            options: CallOptions::default(),
        }
    }

    /// Sets the cache mode for this specific API call (only affects polling mode).
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.options.cache_mode = mode;
        self
    }

    /// Overrides the default retry policy for this specific API call (only affects polling mode).
    #[must_use]
    pub fn retry_policy(mut self, cfg: Option<RetryConfig>) -> Self {
        self.options = self.options.with_retry_policy(cfg);
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

    /// Sets how long a WebSocket stream may receive no frames before treating the socket as dead.
    ///
    /// Yahoo normally sends WebSocket ping frames while quote traffic is idle, so the default waits
    /// for multiple missed heartbeat windows before falling back or closing the stream.
    #[must_use]
    pub const fn websocket_idle_timeout(mut self, timeout: Duration) -> Self {
        self.ws_idle_timeout = timeout;
        self
    }

    /// Sets how long WebSocket startup may spend connecting and subscribing.
    ///
    /// This timeout covers the HTTP upgrade and initial subscription write. It is independent from
    /// [`Self::websocket_idle_timeout`], which applies after the stream is established.
    #[must_use]
    pub const fn websocket_connect_timeout(mut self, timeout: Duration) -> Self {
        self.ws_connect_timeout = timeout;
        self
    }

    fn validated_symbols(&self) -> Result<Vec<String>, crate::core::YfError> {
        if self.symbols.is_empty() {
            return Err(crate::core::YfError::InvalidParams(
                "symbols list cannot be empty".into(),
            ));
        }
        if self.cfg.interval.is_zero() {
            return Err(crate::core::YfError::InvalidParams(
                "stream interval must be greater than zero".into(),
            ));
        }
        if self.ws_idle_timeout.is_zero() {
            return Err(crate::core::YfError::InvalidParams(
                "websocket idle timeout must be greater than zero".into(),
            ));
        }
        if self.ws_connect_timeout.is_zero() {
            return Err(crate::core::YfError::InvalidParams(
                "websocket connect timeout must be greater than zero".into(),
            ));
        }

        normalize_symbols(self.symbols.iter().map(String::as_str))
    }

    /// Starts the stream, returning a handle to control it and a channel receiver for quote updates.
    ///
    /// # Errors
    ///
    /// This method will return an error if no symbols have been added to the builder.
    ///
    /// With [`StreamMethod::Websocket`], this also returns initial WebSocket handshake and
    /// subscription errors. Runtime stream failures after startup close the receiver.
    ///
    /// With [`StreamMethod::WebsocketWithFallback`] and [`StreamMethod::Polling`], this returns
    /// after spawning the background task; connection or polling failures are handled inside the
    /// task.
    pub async fn start(
        &self,
    ) -> Result<(StreamHandle, tokio::sync::mpsc::Receiver<QuoteUpdate>), crate::core::YfError>
    {
        let symbols = self.validated_symbols()?;

        let (tx, rx) = tokio::sync::mpsc::channel::<QuoteUpdate>(1024);
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let (startup_tx, startup_rx) = match self.method {
            StreamMethod::Websocket => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                (Some(tx), Some(rx))
            }
            StreamMethod::WebsocketWithFallback | StreamMethod::Polling => (None, None),
        };

        let join = tokio::spawn({
            let client = self.client.clone();
            let symbols = symbols.clone();
            let cfg = self.cfg.clone();
            let method = self.method;
            let ws_timeouts = WebsocketTimeouts {
                connect: self.ws_connect_timeout,
                idle: self.ws_idle_timeout,
            };

            let mut stop_rx = stop_rx;

            let options = self.options.clone();

            async move {
                match method {
                    StreamMethod::Websocket => {
                        if let Err(e) = run_websocket_stream(
                            &client,
                            symbols,
                            tx,
                            &mut stop_rx,
                            startup_tx,
                            ws_timeouts,
                        )
                        .await
                        {
                            crate::core::logging::trace_warn!(
                                error = %e,
                                "websocket stream failed"
                            );
                            #[cfg(not(feature = "tracing"))]
                            let _ = &e;
                        }
                    }
                    StreamMethod::WebsocketWithFallback => {
                        run_websocket_stream_with_fallback(
                            client,
                            symbols,
                            cfg,
                            tx,
                            &mut stop_rx,
                            &options,
                            ws_timeouts,
                        )
                        .await;
                    }
                    StreamMethod::Polling => {
                        run_polling_stream(client, symbols, cfg, tx, &mut stop_rx, &options).await;
                    }
                }
            }
        });

        if let Some(startup_rx) = startup_rx {
            match startup_rx.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    return Err(YfError::InvalidData(
                        "websocket stream task ended before reporting startup".into(),
                    ));
                }
            }
        }

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

#[derive(Clone, Copy)]
struct WebsocketTimeouts {
    connect: Duration,
    idle: Duration,
}

fn report_websocket_startup_error(
    startup_tx: Option<oneshot::Sender<Result<(), YfError>>>,
    err: YfError,
) -> Result<(), YfError> {
    if let Some(tx) = startup_tx {
        if let Err(Err(err)) = tx.send(Err(err)) {
            return Err(err);
        }
        return Ok(());
    }

    Err(err)
}

fn websocket_remote_closed_error() -> YfError {
    YfError::websocket(WsError::ConnectionClosed)
}

fn websocket_idle_timeout_error(timeout: Duration) -> YfError {
    YfError::websocket(WsError::Io(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!("websocket stream received no frames for {timeout:?}"),
    )))
}

fn websocket_connect_timeout_error(timeout: Duration) -> YfError {
    YfError::websocket(WsError::Io(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!("websocket startup did not complete within {timeout:?}"),
    )))
}

fn websocket_stop_requested(stop_rx: &mut oneshot::Receiver<()>) -> bool {
    !matches!(stop_rx.try_recv(), Err(oneshot::error::TryRecvError::Empty))
}

fn websocket_http_transport_error(err: &reqwest::Error) -> YfError {
    YfError::websocket(WsError::Io(std::io::Error::other(
        RedactedHttpError::new(err).to_string(),
    )))
}

fn websocket_http_upgrade_url(base: &Url) -> Result<Url, YfError> {
    let mut url = base.clone();
    let scheme = match base.scheme() {
        "wss" => "https",
        "ws" => "http",
        scheme => {
            return Err(YfError::InvalidParams(format!(
                "unsupported websocket URL scheme: {scheme}"
            )));
        }
    };

    url.set_scheme(scheme)
        .map_err(|()| YfError::InvalidParams(format!("invalid websocket URL: {base}")))?;
    Ok(url)
}

async fn connect_websocket_stream(
    client: &YfClient,
) -> Result<WebSocketStream<reqwest::Upgraded>, YfError> {
    let base = client.base_stream();
    let upgrade_url = websocket_http_upgrade_url(base)?;
    let ws_key = generate_key();

    let response = client
        .http()
        .get(upgrade_url)
        .version(Version::HTTP_11)
        .header(ORIGIN, "https://finance.yahoo.com")
        .header(USER_AGENT, client.user_agent())
        .header(UPGRADE, "websocket")
        .header(CONNECTION, "Upgrade")
        .header(SEC_WEBSOCKET_KEY, ws_key.as_str())
        .header(SEC_WEBSOCKET_VERSION, "13")
        .send()
        .await
        .map_err(|err| websocket_http_transport_error(&err))?;

    let status = response.status();
    if status != StatusCode::SWITCHING_PROTOCOLS {
        return Err(YfError::Status {
            status: status.as_u16(),
            url: RedactedUrl::new(base).to_string(),
        });
    }

    let expected_accept = derive_accept_key(ws_key.as_bytes());
    if response
        .headers()
        .get(SEC_WEBSOCKET_ACCEPT)
        .is_none_or(|accept| accept != expected_accept.as_str())
    {
        return Err(YfError::websocket(WsError::Protocol(
            tokio_tungstenite::tungstenite::error::ProtocolError::SecWebSocketAcceptKeyMismatch,
        )));
    }

    let upgraded = response
        .upgrade()
        .await
        .map_err(|err| websocket_http_transport_error(&err))?;
    Ok(WebSocketStream::from_raw_socket(upgraded, Role::Client, None).await)
}

#[allow(clippy::too_many_lines)]
async fn run_websocket_stream(
    client: &YfClient,
    symbols: Vec<String>,
    tx: mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut oneshot::Receiver<()>,
    startup_tx: Option<oneshot::Sender<Result<(), YfError>>>,
    timeouts: WebsocketTimeouts,
) -> Result<(), YfError> {
    let startup = async {
        let ws_stream = connect_websocket_stream(client).await?;
        let (mut write, read) = ws_stream.split();

        let sub_msg = serde_json::to_string(&WsSubscribe {
            subscribe: &symbols,
        })
        .map_err(YfError::json)?;
        write
            .send(WsMessage::Text(sub_msg.into()))
            .await
            .map_err(YfError::websocket)?;

        Ok((write, read))
    };

    let startup_result = select! {
        result = tokio::time::timeout(timeouts.connect, startup) => {
            result.unwrap_or_else(|_| Err(websocket_connect_timeout_error(timeouts.connect)))
        },
        _ = &mut *stop_rx => return Ok(()),
    };

    let (mut write, mut read) = match startup_result {
        Ok(parts) => parts,
        Err(err) => return report_websocket_startup_error(startup_tx, err),
    };

    if let Some(tx) = startup_tx
        && tx.send(Ok(())).is_err()
    {
        return Ok(());
    }

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
                                    crate::core::logging::trace_warn!(
                                        error = %e,
                                        "failed to write stream fixture"
                                    );
                                    #[cfg(not(feature = "tracing"))]
                                    let _ = e;
                                }
                                recorded = true;
                            }
                        }

                        match decode_ws_pricing(&text) {
                            Ok(ticker) => {
                                if let Some(update) = map_ws_pricing_to_update(client, &ticker)
                                    && tx.send(update).await.is_err() { return Ok(()); }
                            },
                            Err(e) => {
                                crate::core::logging::trace_debug!(
                                    error = %e,
                                    "websocket text frame decode failed"
                                );
                                #[cfg(not(feature = "tracing"))]
                                let _ = e;
                                // Non-price frames (acks/heartbeats) may lack "message"; ignore.
                            }
                        }
                    }
                    Some(Ok(WsMessage::Binary(bin))) => {
                        // Try to interpret as UTF-8 JSON-wrapped base64 first
                        let handled = if let Ok(as_text) = std::str::from_utf8(&bin) {
                            if let Ok(ticker) = decode_ws_pricing(as_text) {
                                if let Some(update) = map_ws_pricing_to_update(client, &ticker)
                                    && tx.send(update).await.is_err() { return Ok(()); }
                                true
                            } else { false }
                        } else { false };
                        // If not handled, treat as raw protobuf bytes
                        if !handled {
                            match wire_ws::PricingData::decode(&*bin) {
                                Ok(ticker) => {
                                    if let Some(update) = map_ws_pricing_to_update(client, &ticker)
                                        && tx.send(update).await.is_err() { return Ok(()); }
                                }
                                Err(e) => {
                                    crate::core::logging::trace_debug!(
                                        error = %e,
                                        "websocket binary frame decode failed"
                                    );
                                    #[cfg(not(feature = "tracing"))]
                                    let _ = e;
                                }
                            }
                        }
                    }
                    // Tungstenite queues ping replies automatically; keep polling so they flush.
                    Some(Ok(WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Frame(_))) => {}
                    Some(Ok(WsMessage::Close(_))) => {
                        write.flush().await.map_err(YfError::websocket)?;
                        if websocket_stop_requested(stop_rx) {
                            break;
                        }
                        return Err(websocket_remote_closed_error());
                    }
                    Some(Err(e)) => return Err(YfError::websocket(e)),
                    None => {
                        if websocket_stop_requested(stop_rx) {
                            break;
                        }
                        return Err(websocket_remote_closed_error());
                    }
                }
            },
            // Yahoo sends ping frames during idle periods. Missing several of those likely means
            // the TCP/WebSocket connection is half-open, so reconnect or fall back to polling.
            () = tokio::time::sleep(timeouts.idle) => {
                return Err(websocket_idle_timeout_error(timeouts.idle));
            },
            _ = &mut *stop_rx => {
                break;
            }
        }
    }
    Ok(())
}

async fn run_websocket_stream_with_fallback(
    client: crate::core::YfClient,
    symbols: Vec<String>,
    cfg: StreamConfig,
    tx: tokio::sync::mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut tokio::sync::oneshot::Receiver<()>,
    options: &CallOptions,
    timeouts: WebsocketTimeouts,
) {
    let mut ticker = tokio::time::interval(cfg.interval);
    let mut last_price: HashMap<String, Option<Decimal>> = HashMap::new();
    let mut websocket_failures = 0_u32;

    loop {
        if tx.is_closed() || websocket_stop_requested(stop_rx) {
            break;
        }

        match run_websocket_stream(
            &client,
            symbols.clone(),
            tx.clone(),
            stop_rx,
            None,
            timeouts,
        )
        .await
        {
            Ok(()) => break,
            Err(e) => {
                websocket_failures = websocket_failures.saturating_add(1);
                crate::core::logging::trace_warn!(
                    error = %e,
                    "websocket stream failed; polling before reconnect"
                );
                #[cfg(not(feature = "tracing"))]
                let _ = &e;
            }
        }

        let reconnect_delay = websocket_reconnect_backoff(cfg.interval, websocket_failures);
        let reconnect = PollReconnectContext {
            client: &client,
            symbols: &symbols,
            cfg: &cfg,
            tx: &tx,
            options,
            delay: reconnect_delay,
        };
        if !poll_until_websocket_reconnect(reconnect, stop_rx, &mut last_price, &mut ticker).await {
            break;
        }
    }
}

fn websocket_reconnect_backoff(interval: Duration, consecutive_failures: u32) -> Duration {
    let base = interval.max(Duration::from_millis(1));
    let exponent = consecutive_failures.saturating_sub(1).min(16);
    base.saturating_mul(1_u32 << exponent)
        .min(MAX_WEBSOCKET_RECONNECT_BACKOFF)
}

struct PollReconnectContext<'a> {
    client: &'a crate::core::YfClient,
    symbols: &'a [String],
    cfg: &'a StreamConfig,
    tx: &'a tokio::sync::mpsc::Sender<QuoteUpdate>,
    options: &'a CallOptions,
    delay: Duration,
}

async fn poll_until_websocket_reconnect(
    reconnect: PollReconnectContext<'_>,
    stop_rx: &mut tokio::sync::oneshot::Receiver<()>,
    last_price: &mut HashMap<String, Option<Decimal>>,
    ticker: &mut tokio::time::Interval,
) -> bool {
    let reconnect_at = tokio::time::Instant::now() + reconnect.delay;

    loop {
        tokio::select! {
            () = tokio::time::sleep_until(reconnect_at) => return true,
            _ = ticker.tick() => {
                if reconnect.tx.is_closed() {
                    return false;
                }
                if !poll_stream_once(
                    reconnect.client,
                    reconnect.symbols,
                    reconnect.cfg,
                    reconnect.tx,
                    stop_rx,
                    reconnect.options,
                    last_price,
                ).await {
                    return false;
                }
            },
            _ = &mut *stop_rx => return false,
        }
    }
}

fn decode_ws_pricing(text: &str) -> Result<wire_ws::PricingData, YfError> {
    let s = text.trim();
    let b64_cow: Cow<str> = if s.starts_with('{') {
        match serde_json::from_str::<serde_json::Value>(s) {
            Ok(v) => {
                let msg = v.get("message").and_then(|m| m.as_str()).ok_or_else(|| {
                    YfError::MissingData("ws json message missing 'message' field".into())
                })?;
                Cow::Owned(msg.to_string())
            }
            Err(_) => Cow::Borrowed(s),
        }
    } else {
        Cow::Borrowed(s)
    };
    let decoded = general_purpose::STANDARD
        .decode(b64_cow.as_ref())
        .map_err(YfError::base64)?;
    let ticker = wire_ws::PricingData::decode(&*decoded).map_err(YfError::protobuf)?;
    Ok(ticker)
}

fn resolve_stream_instrument(
    client: &YfClient,
    symbol: &str,
    kind: Option<AssetKind>,
) -> Option<Instrument> {
    if let Some(instrument) = client.cached_instrument(symbol) {
        return Some(instrument);
    }

    let should_cache = kind.is_some();
    let kind = kind.unwrap_or_else(untyped_stream_asset_kind);
    let Ok(instrument) = Instrument::from_symbol(symbol, kind) else {
        crate::core::logging::trace_debug!(
            symbol = %symbol,
            "skipping stream update with invalid symbol"
        );
        return None;
    };

    if should_cache {
        client.store_instrument(symbol.to_string(), instrument.clone());
    }

    Some(instrument)
}

fn ws_pricing_timestamp(ticker: &wire_ws::PricingData) -> Result<DateTime<Utc>, YfError> {
    DateTime::from_timestamp_millis(ticker.time).ok_or_else(|| {
        YfError::InvalidParams(format!(
            "Invalid timestamp in stream message: {}",
            ticker.time
        ))
    })
}

fn ws_pricing_to_update(
    ticker: &wire_ws::PricingData,
    instrument: Instrument,
    timestamp: DateTime<Utc>,
    currency_unit: &ResolvedCurrencyUnit,
    volume: Option<QuantityAmount>,
) -> QuoteUpdate {
    QuoteUpdate {
        instrument,
        currency: currency_unit.currency().clone(),
        price: ws_price_from_f32(ticker.price, ticker.price_hint, currency_unit),
        previous_close: ws_price_from_f32(ticker.previous_close, ticker.price_hint, currency_unit),
        volume,
        ts: timestamp,
        provider: (),
    }
}

fn ws_currency_unit(ticker: &wire_ws::PricingData) -> Option<ResolvedCurrencyUnit> {
    let code = nonempty_str(&ticker.currency)
        .or_else(|| nonempty_str(&ticker.exchange).and_then(yahoo_exchange_to_listing_currency))?;
    ResolvedCurrencyUnit::from_code(code)
}

fn nonempty_str(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

const fn stream_quote_type_to_asset_kind(quote_type: i32) -> Option<AssetKind> {
    match quote_type {
        8 => Some(AssetKind::Equity),
        9 => Some(AssetKind::Index),
        11 | 12 | 20 => Some(AssetKind::Fund),
        13 => Some(AssetKind::Option),
        14 => Some(AssetKind::Forex),
        15 => Some(AssetKind::Warrant),
        17 => Some(AssetKind::Bond),
        18 => Some(AssetKind::Future),
        23 => Some(AssetKind::Commodity),
        41 => Some(AssetKind::Crypto),
        _ => None,
    }
}

fn ws_price_from_f32(
    value: f32,
    price_hint: i64,
    currency_unit: &ResolvedCurrencyUnit,
) -> Option<PriceAmount> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let mut value = decimal_from_f32(value)?;
    if let Ok(scale) = u32::try_from(price_hint) {
        value = value.round_dp(scale.min(28));
    }
    currency_unit.price_amount_from_decimal(value)
}

fn map_ws_pricing_to_update(
    client: &YfClient,
    ticker: &wire_ws::PricingData,
) -> Option<QuoteUpdate> {
    let instrument = resolve_stream_instrument(
        client,
        &ticker.id,
        stream_quote_type_to_asset_kind(ticker.quote_type),
    )?;
    let timestamp = match ws_pricing_timestamp(ticker) {
        Ok(timestamp) => timestamp,
        Err(error) => {
            crate::core::logging::trace_debug!(
                error = %error,
                timestamp_millis = ticker.time,
                symbol = %ticker.id,
                "skipping websocket update with invalid timestamp"
            );
            #[cfg(not(feature = "tracing"))]
            let _ = error;
            return None;
        }
    };
    let Some(currency_unit) = ws_currency_unit(ticker) else {
        crate::core::logging::trace_debug!(
            symbol = %ticker.id,
            "skipping websocket update without usable currency"
        );
        return None;
    };
    let volume = quantity_from_i64(ticker.day_volume);

    Some(ws_pricing_to_update(
        ticker,
        instrument,
        timestamp,
        &currency_unit,
        volume,
    ))
}

/// Decodes a single base64-encoded protobuf message from the Yahoo Finance WebSocket stream.
#[cfg(any(test, feature = "test-mode"))]
#[doc(hidden)]
pub fn decode_and_map_message(text: &str) -> Result<QuoteUpdate, YfError> {
    let ticker = decode_ws_pricing(text)?;
    let kind = stream_quote_type_to_asset_kind(ticker.quote_type)
        .unwrap_or_else(untyped_stream_asset_kind);
    let instrument = Instrument::from_symbol(&ticker.id, kind)
        .map_err(|_| YfError::InvalidParams(format!("ws symbol invalid: {}", ticker.id)))?;

    let timestamp = match ws_pricing_timestamp(&ticker) {
        Ok(timestamp) => timestamp,
        Err(error) => {
            crate::core::logging::trace_warn!(
                error = %error,
                timestamp_millis = ticker.time,
                symbol = %ticker.id,
                "received websocket update with invalid timestamp"
            );
            #[cfg(not(feature = "tracing"))]
            let _ = &error;
            return Err(error);
        }
    };

    let currency_unit = ws_currency_unit(&ticker).ok_or_else(|| {
        YfError::InvalidData(format!(
            "websocket update missing usable currency for {}",
            ticker.id
        ))
    })?;

    Ok(ws_pricing_to_update(
        &ticker,
        instrument,
        timestamp,
        &currency_unit,
        quantity_from_i64(ticker.day_volume),
    ))
}

async fn run_polling_stream(
    client: crate::core::YfClient,
    symbols: Vec<String>,
    cfg: StreamConfig,
    tx: tokio::sync::mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut tokio::sync::oneshot::Receiver<()>,
    options: &CallOptions,
) {
    let mut ticker = tokio::time::interval(cfg.interval);
    let mut last_price: HashMap<String, Option<Decimal>> = HashMap::new();

    loop {
        if !wait_for_poll_tick(&mut ticker, &tx, stop_rx).await {
            break;
        }
        if !poll_stream_once(
            &client,
            &symbols,
            &cfg,
            &tx,
            stop_rx,
            options,
            &mut last_price,
        )
        .await
        {
            break;
        }
    }
}

async fn wait_for_poll_tick(
    ticker: &mut tokio::time::Interval,
    tx: &tokio::sync::mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut tokio::sync::oneshot::Receiver<()>,
) -> bool {
    tokio::select! {
        _ = ticker.tick() => !tx.is_closed(),
        _ = &mut *stop_rx => false,
    }
}

async fn poll_stream_once(
    client: &crate::core::YfClient,
    symbols: &[String],
    cfg: &StreamConfig,
    tx: &tokio::sync::mpsc::Sender<QuoteUpdate>,
    stop_rx: &mut tokio::sync::oneshot::Receiver<()>,
    options: &CallOptions,
    last_price: &mut HashMap<String, Option<Decimal>>,
) -> bool {
    if tx.is_closed() {
        return false;
    }

    let symbol_slices: Vec<&str> = symbols.iter().map(AsRef::as_ref).collect();
    let fetch = crate::core::quotes::fetch_v7_quotes(client, &symbol_slices, options);
    let quotes = tokio::select! {
        result = fetch => result,
        _ = &mut *stop_rx => return false,
    };

    match quotes {
        Ok(quotes) => handle_polling_quotes(client, tx, cfg.diff_only, last_price, quotes).await,
        Err(e) => {
            crate::core::logging::trace_debug!(
                error = %e,
                "polling stream quote fetch failed"
            );
            #[cfg(not(feature = "tracing"))]
            let _ = e;
            !tx.is_closed()
        }
    }
}

async fn handle_polling_quotes(
    client: &crate::core::YfClient,
    tx: &tokio::sync::mpsc::Sender<QuoteUpdate>,
    diff_only: bool,
    last_price: &mut HashMap<String, Option<Decimal>>,
    quotes: Vec<crate::core::quotes::V7QuoteNode>,
) -> bool {
    for q in quotes {
        let ts = q
            .regular_market_time
            .as_ref()
            .and_then(|t| DateTime::from_timestamp(*t, 0))
            .unwrap_or_else(Utc::now);
        let sym_s = q.symbol.as_ref().cloned().unwrap_or_default();
        let lp = q
            .regular_market_price
            .as_ref()
            .map(|value| value.into_decimal())
            .or_else(|| {
                q.regular_market_previous_close
                    .as_ref()
                    .map(|value| value.into_decimal())
            });

        let price_changed = if diff_only {
            last_price.get(&sym_s) != Some(&lp)
        } else {
            true
        };

        if diff_only && !price_changed {
            continue;
        }

        let Some(currency_unit) = q
            .currency
            .as_ref()
            .map(String::as_str)
            .and_then(ResolvedCurrencyUnit::from_code)
        else {
            crate::core::logging::trace_debug!(
                symbol = %sym_s,
                "skipping polling stream update without usable currency"
            );
            continue;
        };
        let kind = q
            .quote_type
            .as_ref()
            .map(String::as_str)
            .and_then(|value| parse_yahoo_quote_type(value).ok());
        let Some(instrument) = resolve_stream_instrument(client, &sym_s, kind) else {
            continue;
        };
        if tx
            .send(QuoteUpdate {
                instrument,
                currency: currency_unit.currency().clone(),
                price: lp.and_then(|value| currency_unit.price_amount_from_decimal(value)),
                previous_close: q.regular_market_previous_close.as_ref().and_then(|value| {
                    currency_unit.price_amount_from_decimal(value.into_decimal())
                }),
                volume: q
                    .regular_market_volume
                    .as_ref()
                    .map(|v| v.into_u64())
                    .and_then(quantity_from_u64),
                ts,
                provider: (),
            })
            .await
            .is_err()
        {
            return false;
        }
        if diff_only {
            last_price.insert(sym_s, lp);
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{MAX_WEBSOCKET_RECONNECT_BACKOFF, stream_quote_type_to_asset_kind};
    use crate::core::currency_resolver::ResolvedCurrencyUnit;
    use paft::Decimal;
    use paft::domain::AssetKind;
    use std::time::Duration;

    #[test]
    fn stream_quote_type_to_asset_kind_maps_known_yahoo_codes() {
        for (code, expected) in [
            (8, AssetKind::Equity),
            (9, AssetKind::Index),
            (11, AssetKind::Fund),
            (12, AssetKind::Fund),
            (13, AssetKind::Option),
            (14, AssetKind::Forex),
            (15, AssetKind::Warrant),
            (17, AssetKind::Bond),
            (18, AssetKind::Future),
            (20, AssetKind::Fund),
            (23, AssetKind::Commodity),
            (41, AssetKind::Crypto),
        ] {
            assert_eq!(stream_quote_type_to_asset_kind(code), Some(expected));
        }
    }

    #[test]
    fn stream_quote_type_to_asset_kind_rejects_non_instrument_codes() {
        assert_eq!(stream_quote_type_to_asset_kind(0), None);
        assert_eq!(stream_quote_type_to_asset_kind(7), None);
        assert_eq!(stream_quote_type_to_asset_kind(1000), None);
        assert_eq!(stream_quote_type_to_asset_kind(i32::MAX), None);
    }

    #[test]
    fn websocket_reconnect_backoff_grows_and_caps() {
        let base = Duration::from_millis(40);

        assert_eq!(super::websocket_reconnect_backoff(base, 1), base);
        assert_eq!(
            super::websocket_reconnect_backoff(base, 2),
            Duration::from_millis(80)
        );
        assert_eq!(
            super::websocket_reconnect_backoff(base, u32::MAX),
            MAX_WEBSOCKET_RECONNECT_BACKOFF
        );
    }

    #[test]
    fn websocket_price_uses_price_hint_to_trim_f32_noise() {
        let unit = ResolvedCurrencyUnit::from_code("USD").expect("valid currency");
        let noisy_xrp_price = f32::from_bits(0x3f8b_e76d);

        let price =
            super::ws_price_from_f32(noisy_xrp_price, 4, &unit).expect("positive finite price");

        assert_eq!(price.as_decimal(), &Decimal::new(10_930, 4));
    }

    #[test]
    fn websocket_price_hint_rounds_before_minor_unit_scaling() {
        let unit = ResolvedCurrencyUnit::from_code("GBp").expect("valid minor-unit currency");

        let price = super::ws_price_from_f32(123.45, 2, &unit).expect("positive finite price");

        assert_eq!(price.as_decimal(), &Decimal::new(12_345, 4));
    }
}
