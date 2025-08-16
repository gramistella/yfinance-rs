use serde::Deserialize;
use url::Url;

use crate::{YfClient, YfError, core::net};

use super::model::Quote;

/* ---------------- Public: fetch a single quote ---------------- */

pub(crate) async fn fetch_quote(
    client: &mut YfClient,
    base: &Url,
    symbol: &str,
) -> Result<Quote, YfError> {
    let http = client.http().clone();

    // Attempt without auth
    let mut url = base.clone();
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("symbols", symbol);
    }

    let mut resp = http
        .get(url.clone())
        .header("accept", "application/json")
        .send()
        .await?;

    if resp.status().is_success() {
        return parse_quote_from_response(resp, symbol).await;
    }

    let code = resp.status().as_u16();
    if code != 401 && code != 403 {
        return Err(YfError::Status {
            status: code,
            url: url.to_string(),
        });
    }

    // Retry with crumb
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

    parse_quote_from_response(resp, symbol).await
}

/* ---------------- Internal helpers ---------------- */

async fn parse_quote_from_response(
    resp: reqwest::Response,
    symbol: &str,
) -> Result<Quote, YfError> {
    let body = net::get_text(resp, "quote_v7", symbol, "json").await?;
    let env: V7Envelope =
        serde_json::from_str(&body).map_err(|e| YfError::Data(format!("quote json parse: {e}")))?;

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
