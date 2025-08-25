use serde::Deserialize;
use url::Url;

use crate::{
    YfClient, YfError,
    core::{
        client::{CacheMode, RetryConfig},
        net,
    },
};

use super::model::Quote;

async fn parse_quote_from_body(body: &str, symbol: &str) -> Result<Quote, YfError> {
    let env: V7Envelope =
        serde_json::from_str(body).map_err(|e| YfError::Data(format!("quote json parse: {e}")))?;
    let result = env
        .quote_response
        .and_then(|qr| qr.result)
        .and_then(|mut v| v.pop())
        .ok_or_else(|| YfError::Data("empty quote result".into()))?;

    Ok(Quote {
        symbol: result.symbol.unwrap_or_else(|| symbol.to_string()),
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

/* ---------------- Public: fetch a single quote ---------------- */

pub(crate) async fn fetch_quote(
    client: &mut YfClient,
    base: &Url,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Quote, YfError> {
    let http = client.http().clone();

    let mut url = base.clone();
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("symbols", symbol);
    }

    if cache_mode == CacheMode::Use {
        if let Some(body) = client.cache_get(&url).await {
            return parse_quote_from_body(&body, symbol).await;
        }
    }

    let req = http.get(url.clone()).header("accept", "application/json");
    let mut resp = client.send_with_retry(req, retry_override).await?;

    if resp.status().is_success() {
        let body = net::get_text(resp, "quote_v7", symbol, "json").await?;
        if cache_mode != CacheMode::Bypass {
            client.cache_put(&url, &body, None).await;
        }
        return parse_quote_from_body(&body, symbol).await;
    }

    let code = resp.status().as_u16();
    if code != 401 && code != 403 {
        return Err(YfError::Status {
            status: code,
            url: url.to_string(),
        });
    }

    client.ensure_credentials().await?;
    let crumb = client
        .crumb()
        .ok_or_else(|| YfError::Status {
            status: code,
            url: url.to_string(),
        })?
        .to_string();

    let mut url2 = base.clone();
    {
        let mut qp = url2.query_pairs_mut();
        qp.append_pair("symbols", symbol);
        qp.append_pair("crumb", &crumb);
    }

    let req2 = http.get(url2.clone()).header("accept", "application/json");
    resp = client.send_with_retry(req2, retry_override).await?;

    if !resp.status().is_success() {
        return Err(YfError::Status {
            status: resp.status().as_u16(),
            url: url2.to_string(),
        });
    }

    let body = net::get_text(resp, "quote_v7", symbol, "json").await?;
    if cache_mode != CacheMode::Bypass {
        client.cache_put(&url2, &body, None).await;
    }
    parse_quote_from_body(&body, symbol).await
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

    #[serde(rename = "fullExchangeName")]
    full_exchange_name: Option<String>,
    exchange: Option<String>,
    market: Option<String>,
    #[serde(rename = "marketCapFigureExchange")]
    market_cap_figure_exchange: Option<String>,

    #[serde(rename = "marketState")]
    market_state: Option<String>,
}
