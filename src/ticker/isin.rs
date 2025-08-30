use crate::{
    YfClient, YfError,
    core::{client::RetryConfig, net},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct FlatSuggest {
    #[serde(alias = "Value", alias = "value")]
    value: Option<String>,
    #[serde(alias = "Symbol", alias = "symbol")]
    symbol: Option<String>,
    #[serde(alias = "Isin", alias = "isin", alias = "ISIN")]
    isin: Option<String>,
}

pub(super) async fn fetch_isin(
    client: &YfClient,
    symbol: &str,
    retry_override: Option<&RetryConfig>,
) -> Result<Option<String>, YfError> {
    let Some(body) = fetch_isin_body(client, symbol, retry_override).await? else {
        return Ok(None);
    };

    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");
    let input_norm = normalize_sym(symbol);

    if let Some(isin) = parse_as_json_value(&body, &input_norm, debug) {
        return Ok(Some(isin));
    }

    if let Some(isin) = parse_as_flat_suggest(&body, &input_norm) {
        return Ok(Some(isin));
    }

    if let Some(isin) = scan_raw_body(&body, debug) {
        return Ok(Some(isin));
    }

    if debug {
        eprintln!("YF_DEBUG(isin): No matching ISIN found in any response shape.");
    }
    Ok(None)
}

async fn fetch_isin_body(
    client: &YfClient,
    symbol: &str,
    retry_override: Option<&RetryConfig>,
) -> Result<Option<String>, YfError> {
    let mut url = client.base_insider_search().clone();
    url.query_pairs_mut()
        .append_pair("max_results", "5")
        .append_pair("query", symbol);

    let req = client.http().get(url.clone());
    let resp = client.send_with_retry(req, retry_override).await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    Ok(Some(
        net::get_text(resp, "isin_search", symbol, "json").await?,
    ))
}

fn parse_as_json_value(body: &str, input_norm: &str, debug: bool) -> Option<String> {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(hit) = extract_from_json_value(&val, input_norm) {
            if debug {
                eprintln!("YF_DEBUG(isin): ISIN extracted from JSON structures: {hit}",);
            }
            return Some(hit);
        }
    } else if debug {
        eprintln!("YF_DEBUG(isin): failed to parse JSON response for query '{input_norm}'",);
    }
    None
}

fn parse_as_flat_suggest(body: &str, input_norm: &str) -> Option<String> {
    if let Ok(raw_arr) = serde_json::from_str::<Vec<FlatSuggest>>(body) {
        for r in &raw_arr {
            if let Some(isin) = r.isin.as_deref()
                && looks_like_isin(isin)
                && r.symbol.as_deref().map(normalize_sym) == Some(input_norm.to_string())
            {
                return Some(isin.to_uppercase());
            }
            if let Some(value) = r.value.as_deref() {
                let parts: Vec<String> = value
                    .split('|')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
                if let Some(isin) = pick_from_parts(&parts, input_norm) {
                    return Some(isin);
                }
            }
        }
        for r in &raw_arr {
            if let Some(isin) = r.isin.as_deref()
                && looks_like_isin(isin)
            {
                return Some(isin.to_uppercase());
            }
            if let Some(value) = r.value.as_deref()
                && let Some(tok) = value
                    .split('|')
                    .map(str::trim)
                    .find(|tok| looks_like_isin(tok))
            {
                return Some((*tok).to_uppercase());
            }
        }
    }
    None
}

fn scan_raw_body(body: &str, debug: bool) -> Option<String> {
    let mut token = String::new();
    for ch in body.chars() {
        if ch.is_ascii_alphanumeric() {
            token.push(ch);
            if token.len() > 12 {
                token.remove(0);
            }
            if token.len() == 12 && looks_like_isin(&token) {
                if debug {
                    eprintln!("YF_DEBUG(isin): Fallback raw scan found ISIN: {token}");
                }
                return Some(token.to_uppercase());
            }
        } else {
            token.clear();
        }
    }
    None
}

fn extract_from_json_value(v: &serde_json::Value, target_norm: &str) -> Option<String> {
    let mut arrays: Vec<&serde_json::Value> = Vec::new();

    match v {
        serde_json::Value::Array(_) => arrays.push(v),
        serde_json::Value::Object(map) => {
            for key in [
                "Suggestions",
                "suggestions",
                "items",
                "results",
                "Result",
                "data",
            ] {
                if let Some(val) = map.get(key)
                    && val.is_array()
                {
                    arrays.push(val);
                }
            }
            if arrays.is_empty() {
                for (_, val) in map {
                    if val.is_array() {
                        arrays.push(val);
                    } else if let Some(obj) = val.as_object() {
                        for (_, inner) in obj {
                            if inner.is_array() {
                                arrays.push(inner);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    for arr in arrays {
        if let Some(a) = arr.as_array() {
            for item in a {
                if let Some(obj) = item.as_object() {
                    for k in ["Isin", "isin", "ISIN"] {
                        if let Some(isin_val) = obj.get(k).and_then(|x| x.as_str())
                            && looks_like_isin(isin_val)
                        {
                            let sym = obj
                                .get("Symbol")
                                .and_then(|x| x.as_str())
                                .or_else(|| obj.get("symbol").and_then(|x| x.as_str()))
                                .unwrap_or("");
                            if sym.is_empty() || normalize_sym(sym) == target_norm {
                                return Some(isin_val.to_uppercase());
                            }
                        }
                    }

                    let value_str = obj
                        .get("Value")
                        .and_then(|x| x.as_str())
                        .or_else(|| obj.get("value").and_then(|x| x.as_str()))
                        .unwrap_or("");
                    if !value_str.is_empty() {
                        let parts: Vec<String> = value_str
                            .split('|')
                            .map(|p| p.trim().to_string())
                            .filter(|p| !p.is_empty())
                            .collect();
                        if let Some(isin) = pick_from_parts(&parts, target_norm) {
                            return Some(isin);
                        }
                    }

                    if let Some(sym) = obj
                        .get("Symbol")
                        .and_then(|x| x.as_str())
                        .or_else(|| obj.get("symbol").and_then(|x| x.as_str()))
                        && normalize_sym(sym) == target_norm
                    {
                        for (_k, v) in obj {
                            if let Some(s) = v.as_str()
                                && looks_like_isin(s)
                            {
                                return Some(s.to_uppercase());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn normalize_sym(s: &str) -> String {
    let mut t = s.trim().replace('-', ".");
    for sep in ['.', ':', ' ', '\t', '\n', '\r'] {
        if let Some(idx) = t.find(sep) {
            t.truncate(idx);
            break;
        }
    }
    t.to_ascii_lowercase()
}

fn looks_like_isin(s: &str) -> bool {
    let t = s.trim();
    if t.len() != 12 {
        return false;
    }
    let b = t.as_bytes();
    if !(b[0].is_ascii_alphabetic() && b[1].is_ascii_alphabetic()) {
        return false;
    }
    if !t[2..11].chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    b[11].is_ascii_digit()
}

fn pick_from_parts(parts: &[String], target_norm: &str) -> Option<String> {
    if let Some(first) = parts.first()
        && normalize_sym(first) == target_norm
        && let Some(isin) = parts
            .iter()
            .map(std::string::String::as_str)
            .find(|s| looks_like_isin(s))
    {
        return Some(isin.to_uppercase());
    }

    None
}
