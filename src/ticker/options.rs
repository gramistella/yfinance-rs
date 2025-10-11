use std::str::FromStr;

use serde::Deserialize;
use url::Url;

use crate::{
    YfClient, YfError,
    core::{
        client::{CacheMode, RetryConfig},
        conversions::{f64_to_money_with_currency, i64_to_datetime},
        net,
    },
};
use paft::money::Currency;

use super::model::{OptionChain, OptionContract};
use chrono::{NaiveDate, TimeZone, Utc};

/* ---------------- Public: expirations + chain ---------------- */

pub async fn expiration_dates(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<i64>, YfError> {
    let (body, _used_url) =
        fetch_options_raw(client, symbol, None, cache_mode, retry_override).await?;
    let env: OptEnvelope = serde_json::from_str(&body).map_err(YfError::Json)?;

    let first = env
        .option_chain
        .and_then(|oc| oc.result)
        .and_then(|mut v| v.pop())
        .ok_or_else(|| YfError::MissingData("empty options result".into()))?;

    Ok(first.expiration_dates.unwrap_or_default())
}

pub async fn option_chain(
    client: &YfClient,
    symbol: &str,
    date: Option<i64>,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<OptionChain, YfError> {
    let (body, used_url) =
        fetch_options_raw(client, symbol, date, cache_mode, retry_override).await?;
    let env: OptEnvelope = serde_json::from_str(&body).map_err(YfError::Json)?;

    let first = env
        .option_chain
        .and_then(|oc| oc.result)
        .and_then(|mut v| v.pop())
        .ok_or_else(|| YfError::MissingData("empty options result".into()))?;

    let currency_from_response = currency_from_result(&first);

    let Some(od) = first.options.and_then(|mut v| v.pop()) else {
        return Ok(OptionChain {
            calls: vec![],
            puts: vec![],
        });
    };

    let expiration = od.expiration_date.unwrap_or_else(|| {
        if let Some(q) = used_url.query() {
            for kv in q.split('&') {
                if let Some(v) = kv.strip_prefix("date=")
                    && let Ok(ts) = v.parse::<i64>()
                {
                    return ts;
                }
            }
        }
        0
    });

    let currency = if let Some(currency) = currency_from_response {
        currency
    } else {
        let quote = super::quote::fetch_quote(client, symbol, cache_mode, retry_override).await?;
        quote
            .price
            .as_ref()
            .map(|m| m.currency().clone())
            .or_else(|| quote.previous_close.as_ref().map(|m| m.currency().clone()))
            .ok_or_else(|| {
                YfError::MissingData("unable to determine currency from options or quote".into())
            })?
    };

    let map_side = |side: Option<Vec<OptContractNode>>,
                    currency: &Currency|
     -> Vec<OptionContract> {
        side.unwrap_or_default()
            .into_iter()
            .map(|c| {
                let exp_ts = c.expiration.unwrap_or(expiration);
                let exp_dt = i64_to_datetime(exp_ts);
                let exp_date: NaiveDate = exp_dt.date_naive();

                OptionContract {
                    contract_symbol: c.contract_symbol.unwrap_or_default(),
                    strike: f64_to_money_with_currency(c.strike.unwrap_or(0.0), currency.clone()),
                    price: c
                        .last_price
                        .map(|v| f64_to_money_with_currency(v, currency.clone())),
                    bid: c
                        .bid
                        .map(|v| f64_to_money_with_currency(v, currency.clone())),
                    ask: c
                        .ask
                        .map(|v| f64_to_money_with_currency(v, currency.clone())),
                    volume: c.volume,
                    open_interest: c.open_interest,
                    implied_volatility: c.implied_volatility,
                    in_the_money: c.in_the_money.unwrap_or(false),
                    expiration_date: exp_date,
                    expiration_at: Some(exp_dt),
                    last_trade_at: c
                        .last_trade_date
                        .and_then(|ts| Utc.timestamp_opt(ts, 0).single()),
                    greeks: None,
                }
            })
            .collect()
    };

    Ok(OptionChain {
        calls: map_side(od.calls, &currency),
        puts: map_side(od.puts, &currency),
    })
}

/* ---------------- Internal: raw fetch with auth fallback ---------------- */

async fn fetch_options_raw(
    client: &YfClient,
    symbol: &str,
    date: Option<i64>,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<(String, Url), YfError> {
    let http = client.http().clone();
    let base = client.base_options_v7();

    let mut url = base.join(symbol)?;
    {
        let mut qp = url.query_pairs_mut();
        if let Some(d) = date {
            qp.append_pair("date", &d.to_string());
        }
    }

    if cache_mode == CacheMode::Use
        && let Some(body) = client.cache_get(&url).await
    {
        return Ok((body, url));
    }

    let req = http.get(url.clone()).header("accept", "application/json");
    let mut resp = client.send_with_retry(req, retry_override).await?;

    if resp.status().is_success() {
        let fixture_key = date.map_or_else(|| symbol.to_string(), |d| format!("{symbol}_{d}"));
        let body = net::get_text(resp, "options_v7", &fixture_key, "json").await?;
        if cache_mode != CacheMode::Bypass {
            client.cache_put(&url, &body, None).await;
        }
        return Ok((body, url));
    }

    let code = resp.status().as_u16();
    if code != 401 && code != 403 {
        let url_s = url.to_string();
        return Err(match code {
            404 => YfError::NotFound { url: url_s },
            429 => YfError::RateLimited { url: url_s },
            500..=599 => YfError::ServerError {
                status: code,
                url: url_s,
            },
            _ => YfError::Status {
                status: code,
                url: url_s,
            },
        });
    }

    client.ensure_credentials().await?;
    let crumb = client.crumb().await.ok_or_else(|| YfError::Status {
        status: code,
        url: url.to_string(),
    })?;

    let mut url2 = base.join(symbol)?;
    {
        let mut qp = url2.query_pairs_mut();
        if let Some(d) = date {
            qp.append_pair("date", &d.to_string());
        }
        qp.append_pair("crumb", &crumb);
    }

    let req2 = http.get(url2.clone()).header("accept", "application/json");
    resp = client.send_with_retry(req2, retry_override).await?;

    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        let url_s = url2.to_string();
        return Err(match code {
            404 => YfError::NotFound { url: url_s },
            429 => YfError::RateLimited { url: url_s },
            500..=599 => YfError::ServerError {
                status: code,
                url: url_s,
            },
            _ => YfError::Status {
                status: code,
                url: url_s,
            },
        });
    }

    let fixture_key = date.map_or_else(|| symbol.to_string(), |d| format!("{symbol}_{d}"));
    let body = net::get_text(resp, "options_v7", &fixture_key, "json").await?;
    if cache_mode != CacheMode::Bypass {
        client.cache_put(&url2, &body, None).await;
    }
    Ok((body, url2))
}

/* ---------------- Minimal serde mapping for v7 options ---------------- */

#[derive(Deserialize)]
struct OptEnvelope {
    #[serde(rename = "optionChain")]
    option_chain: Option<OptChainNode>,
}

#[derive(Deserialize)]
struct OptChainNode {
    result: Option<Vec<OptResultNode>>,
    #[allow(dead_code)]
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct OptResultNode {
    #[serde(rename = "expirationDates")]
    expiration_dates: Option<Vec<i64>>,
    quote: Option<OptQuoteNode>,
    options: Option<Vec<OptByDateNode>>,
}

#[derive(Deserialize)]
struct OptQuoteNode {
    currency: Option<String>,
}

#[derive(Deserialize)]
struct OptByDateNode {
    #[serde(rename = "expirationDate")]
    expiration_date: Option<i64>,
    calls: Option<Vec<OptContractNode>>,
    puts: Option<Vec<OptContractNode>>,
}

#[derive(Deserialize)]
struct OptContractNode {
    #[serde(rename = "contractSymbol")]
    contract_symbol: Option<String>,
    #[serde(rename = "expiration")]
    expiration: Option<i64>,
    #[serde(rename = "lastTradeDate")]
    last_trade_date: Option<i64>,
    strike: Option<f64>,
    #[serde(rename = "lastPrice")]
    last_price: Option<f64>,
    bid: Option<f64>,
    ask: Option<f64>,
    volume: Option<u64>,
    #[serde(rename = "openInterest")]
    open_interest: Option<u64>,
    #[serde(rename = "impliedVolatility")]
    implied_volatility: Option<f64>,
    #[serde(rename = "inTheMoney")]
    in_the_money: Option<bool>,
}

fn currency_from_result(node: &OptResultNode) -> Option<Currency> {
    node.quote
        .as_ref()
        .and_then(|q| q.currency.as_deref())
        .and_then(|code| Currency::from_str(code).ok())
}
