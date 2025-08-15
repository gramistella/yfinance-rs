use crate::{YfClient, YfError};
use serde::Deserialize;

/// Postal address for company HQ (when available).
#[derive(Debug, Clone, PartialEq)]
pub struct Address {
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zip: Option<String>,
}

/// Company profile.
#[derive(Debug, Clone, PartialEq)]
pub struct Company {
    pub name: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub website: Option<String>,
    pub address: Option<Address>,
    pub summary: Option<String>,
}

/// Fund profile (e.g., ETF).
#[derive(Debug, Clone, PartialEq)]
pub struct Fund {
    pub name: String,
    pub family: Option<String>,
    pub kind: String, // e.g., "Exchange Traded Fund"
}

/// Unified profile type.
#[derive(Debug, Clone, PartialEq)]
pub enum Profile {
    Company(Company),
    Fund(Fund),
}


fn extract_crumb(body: &str) -> Option<String> {
    // A simple string search for the crumb is often the most reliable method.
    let key = "\"CrumbStore\":{\"crumb\":\"";
    body.find(key).and_then(|start| {
        let remainder = &body[start + key.len()..];
        remainder.find('"').map(|end| {
            let crumb_escaped = &remainder[..end];
            // The crumb can contain unicode escapes like \u002F for a forward slash.
            // A simple replacement is sufficient for our needs.
            crumb_escaped.replace("\\u002F", "/")
        })
    })
}

async fn load_from_scrape(client: &YfClient, symbol: &str) -> Result<Profile, YfError> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");

    let mut url = client.base_quote().join(symbol)?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("p", symbol);
    }
    let quote_page_resp = client.http().get(url.clone()).send().await?;
    if !quote_page_resp.status().is_success() {
        return Err(YfError::Status {
            status: quote_page_resp.status().as_u16(),
            url: url.to_string(),
        });
    }
    let body = crate::net::get_text(quote_page_resp, "profile_html", symbol, "html").await?;

    if debug {
        let _ = debug_dump_html(symbol, &body);
    }

    let json_str = extract_bootstrap_json(&body)?;
    if debug {
        let _ = debug_dump_extracted_json(symbol, &json_str);
    }

    // NOTE: Our extractor may return a QuoteSummaryStore that *doesn't* include `quoteType`.
    // We therefore model it with Option and infer when missing.
    let boot: Bootstrap = serde_json::from_str(&json_str)
        .map_err(|e| YfError::Data(format!("bootstrap json parse: {e}")))?;

    let store = boot.context.dispatcher.stores.quote_summary_store;

    // Name resolution: prefer quoteType.longName/shortName, else price.longName/shortName, else symbol.
    let name = store
        .quote_type
        .as_ref()
        .and_then(|qt| qt.long_name.clone().or(qt.short_name.clone()))
        .or_else(|| {
            store
                .price
                .as_ref()
                .and_then(|p| p.long_name.clone().or(p.short_name.clone()))
        })
        .unwrap_or_else(|| symbol.to_string());

    // Kind resolution:
    // 1) If quoteType.kind exists, use it.
    // 2) Else infer: presence of fundProfile => "ETF", summaryProfile => "EQUITY".
    let inferred_kind = if store.fund_profile.is_some() {
        Some("ETF")
    } else if store.summary_profile.is_some() {
        Some("EQUITY")
    } else {
        None
    };
    let kind = store
        .quote_type
        .as_ref()
        .and_then(|qt| qt.kind.as_deref())
        .or(inferred_kind)
        .unwrap_or("");

    if debug {
        eprintln!(
            "YF_DEBUG [load_from_scrape]: resolved kind=`{}`, name=`{}` (quote_type_present={}, price_present={}, has_summary_profile={}, has_fund_profile={})",
            kind,
            name,
            store.quote_type.is_some(),
            store.price.is_some(),
            store.summary_profile.is_some(),
            store.fund_profile.is_some()
        );
    }

    match kind {
        "EQUITY" => {
            let sp = store
                .summary_profile
                .ok_or_else(|| YfError::Data("summaryProfile missing".into()))?;
            let address = Address {
                street1: sp.address1,
                street2: sp.address2,
                city: sp.city,
                state: sp.state,
                country: sp.country,
                zip: sp.zip,
            };
            Ok(Profile::Company(Company {
                name,
                sector: sp.sector,
                industry: sp.industry,
                website: sp.website,
                summary: sp.long_business_summary,
                address: Some(address),
            }))
        }
        "ETF" => {
            let fp = store
                .fund_profile
                .ok_or_else(|| YfError::Data("fundProfile missing".into()))?;
            Ok(Profile::Fund(Fund {
                name,
                family: fp.family,
                kind: fp.legal_type.unwrap_or_else(|| "Fund".to_string()),
            }))
        }
        other => Err(YfError::Data(format!(
            "unsupported or unknown quoteType: {other}"
        ))),
    }
}

fn extract_bootstrap_json(body: &str) -> Result<String, YfError> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");
    let trunc = |s: &str, n: usize| -> String {
        if s.len() <= n {
            s.to_string()
        } else {
            let mut out = String::with_capacity(n + 16);
            out.push_str(&s[..n]);
            out.push_str(" …[trunc]");
            out
        }
    };

    if debug {
        eprintln!(
            "YF_DEBUG [extract_bootstrap_json]: starting, body.len()={}",
            body.len()
        );
    }

    /* Strategy A: legacy root.App.main = {...}; */
    if debug {
        eprintln!("YF_DEBUG [extract_bootstrap_json]: Strategy A (root.App.main)...");
    }
    if let Some(start) = body.find("root.App.main") {
        let after = &body[start..];
        if let Some(eq) = after.find('=') {
            let mut payload = &after[eq + 1..];
            payload = payload.trim_start();
            let end_script = payload.find("</script>").unwrap_or(payload.len());
            let segment = &payload[..end_script];
            if let Some(semi) = segment.rfind(';') {
                let json_str = segment[..semi].trim();
                if debug {
                    eprintln!(
                        "YF_DEBUG [extract_bootstrap_json]: Strategy A hit; json.len={} preview=`{}`",
                        json_str.len(),
                        trunc(json_str, 160)
                    );
                }
                return Ok(json_str.to_string());
            }
        }
    }

    /* Strategy B: find a literal "QuoteSummaryStore": { ... } object and wrap it */
    if debug {
        eprintln!("YF_DEBUG [extract_bootstrap_json]: Strategy B (QuoteSummaryStore literal)...");
    }
    let key = "\"QuoteSummaryStore\"";
    if let Some(pos) = body.find(key) {
        let after = &body[pos + key.len()..];
        if let Some(brace_rel) = after.find('{') {
            let obj_start = pos + key.len() + brace_rel;
            if let Some(obj_end) = find_matching_brace(body, obj_start) {
                let obj = &body[obj_start..=obj_end];
                let wrapped = format!(
                    r#"{{"context":{{"dispatcher":{{"stores":{{"QuoteSummaryStore":{obj}}}}}}}}}"#
                );
                if debug {
                    eprintln!(
                        "YF_DEBUG [extract_bootstrap_json]: Strategy B hit; obj.len={} preview=`{}`",
                        obj.len(),
                        trunc(obj, 160)
                    );
                }
                return Ok(wrapped);
            } else if debug {
                eprintln!("YF_DEBUG [extract_bootstrap_json]: Strategy B found start but failed to match closing brace.");
            }
        }
    }

    /* Strategy C: SvelteKit data-sveltekit-fetched blobs.
       Two sub-forms:
         - Older: JSON array with nodes[*].data
         - Modern: JSON object with a 'body' field (stringified JSON or inline object)
    */
    if debug {
        eprintln!("YF_DEBUG [extract_bootstrap_json]: Strategy C (SvelteKit fetched JSON)...");
    }
    let scripts = iter_json_scripts(body);
    if debug {
        eprintln!(
            "YF_DEBUG [extract_bootstrap_json]: Strategy C inspecting {} JSON scripts...",
            scripts.len()
        );
    }

    for (i, (tag_attrs, inner_json)) in scripts.iter().enumerate() {
        let is_svelte = tag_attrs.contains("data-sveltekit-fetched");
        if !is_svelte {
            continue;
        }

        if debug {
            eprintln!(
                "YF_DEBUG [extract_bootstrap_json]: C[{}] attrs=`{}` inner.len={} preview=`{}`",
                i,
                trunc(tag_attrs, 160),
                inner_json.len(),
                trunc(inner_json, 120)
            );
        }

        // C1) Older SvelteKit: inner_json is an array having nodes[*].data
        if let Ok(outer_array) = serde_json::from_str::<Vec<Value>>(inner_json) {
            if debug {
                eprintln!(
                    "YF_DEBUG [extract_bootstrap_json]: C[{}] parsed as ARRAY (len={})",
                    i,
                    outer_array.len()
                );
            }
            for (ai, outer_obj) in outer_array.into_iter().enumerate() {
                if let Some(nodes) = outer_obj.get("nodes").and_then(|n| n.as_array()) {
                    if debug {
                        eprintln!("YF_DEBUG [extract_bootstrap_json]: C[{}][{}] nodes.len={}", i, ai, nodes.len());
                    }
                    for (ni, node) in nodes.iter().enumerate() {
                        if let Some(data) = node.get("data") {
                            if let Some(store_like) =
                                extract_store_like_from_quote_summary_value(data)
                            {
                                let wrapped = wrap_store_like(store_like)?;
                                if debug {
                                    eprintln!(
                                        "YF_DEBUG [extract_bootstrap_json]: C[{}][{}] SUCCESS via nodes[{}].data -> wrapped.len={}",
                                        i, ai, ni, wrapped.len()
                                    );
                                }
                                return Ok(wrapped);
                            } else if debug {
                                eprintln!(
                                    "YF_DEBUG [extract_bootstrap_json]: C[{}][{}] nodes[{}].data did not match expected shape.",
                                    i, ai, ni
                                );
                            }
                        }
                    }
                }
            }
        }

        // C2) Modern SvelteKit: inner_json is an object with "body" that is either JSON string or object.
        let parsed_obj = match serde_json::from_str::<Value>(inner_json) {
            Ok(v @ Value::Object(_)) => Some(v),
            Ok(_) => None,
            Err(e) => {
                if debug {
                    eprintln!(
                        "YF_DEBUG [extract_bootstrap_json]: C[{}] parse-as-OBJECT failed: {}",
                        i, e
                    );
                }
                None
            }
        };

        if let Some(mut outer_obj) = parsed_obj {
            let body_val_opt = {
                if let Some(b) = outer_obj.get_mut("body") {
                    Some(b.take())
                } else {
                    None
                }
            };

            if body_val_opt.is_none() && debug {
                eprintln!("YF_DEBUG [extract_bootstrap_json]: C[{}] no 'body' field.", i);
            }

            if let Some(body_val) = body_val_opt {
                let payload_opt = match body_val {
                    Value::String(s) => {
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: C[{}] body is STRING (len={}), preview=`{}`",
                                i,
                                s.len(),
                                trunc(&s, 160)
                            );
                        }
                        serde_json::from_str::<Value>(&s).ok()
                    }
                    Value::Object(_) | Value::Array(_) => {
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: C[{}] body is already JSON (type={}).",
                                i,
                                match &body_val {
                                    Value::Object(_) => "object",
                                    Value::Array(_) => "array",
                                    _ => "other",
                                }
                            );
                        }
                        Some(body_val)
                    }
                    other => {
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: C[{}] body is unsupported type: {:?}",
                                i, other
                            );
                        }
                        None
                    }
                };

                if let Some(payload) = payload_opt {
                    if debug {
                        let preview = match &payload {
                            Value::String(s) => trunc(s, 160),
                            _ => {
                                let s = serde_json::to_string(&payload).unwrap_or_default();
                                trunc(&s, 160)
                            }
                        };
                        eprintln!(
                            "YF_DEBUG [extract_bootstrap_json]: C[{}] payload ready; preview=`{}`",
                            i, preview
                        );
                    }

                    if let Some(qss) = find_quote_summary_store_in_value(&payload) {
                        let store_like = normalize_store_like(qss.clone());
                        let wrapped = wrap_store_like(store_like)?;
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: C[{}] SUCCESS via QuoteSummaryStore path; wrapped.len={}",
                                i, wrapped.len()
                            );
                        }
                        return Ok(wrapped);
                    }

                    if let Some(qs_val) = find_quote_summary_value_in_value(&payload) {
                        if let Some(store_like) =
                            extract_store_like_from_quote_summary_value(qs_val)
                        {
                            let wrapped = wrap_store_like(store_like)?;
                            if debug {
                                eprintln!(
                                    "YF_DEBUG [extract_bootstrap_json]: C[{}] SUCCESS via quoteSummary->result; wrapped.len={}",
                                    i, wrapped.len()
                                );
                            }
                            return Ok(wrapped);
                        } else if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: C[{}] quoteSummary present but missing expected fields.",
                                i
                            );
                        }
                    } else if debug {
                        eprintln!(
                            "YF_DEBUG [extract_bootstrap_json]: C[{}] no quoteSummary or QuoteSummaryStore found in payload.",
                            i
                        );
                    }
                } else if debug {
                    eprintln!(
                        "YF_DEBUG [extract_bootstrap_json]: C[{}] body -> payload parse failed or unsupported.",
                        i
                    );
                }
            }
        }
    }

    /* Strategy D: scan ALL application/json scripts generically (incl. "__NEXT_DATA__" if any) */
    if debug {
        eprintln!("YF_DEBUG [extract_bootstrap_json]: Strategy D (generic JSON scan)...");
    }
    for (i, (_attrs, inner_json)) in scripts.iter().enumerate() {
        let val = match serde_json::from_str::<Value>(inner_json) {
            Ok(v) => v,
            Err(e) => {
                if debug {
                    eprintln!(
                        "YF_DEBUG [extract_bootstrap_json]: D[{}] parse failed: {} (preview=`{}`)",
                        i,
                        e,
                        trunc(inner_json, 120)
                    );
                }
                continue;
            }
        };

        if let Some(qss) = find_quote_summary_store_in_value(&val) {
            let store_like = normalize_store_like(qss.clone());
            let wrapped = wrap_store_like(store_like)?;
            if debug {
                eprintln!(
                    "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via QuoteSummaryStore; wrapped.len={}",
                    i, wrapped.len()
                );
            }
            return Ok(wrapped);
        }

        if let Some(qs_val) = find_quote_summary_value_in_value(&val) {
            if let Some(store_like) = extract_store_like_from_quote_summary_value(qs_val) {
                let wrapped = wrap_store_like(store_like)?;
                if debug {
                    eprintln!(
                        "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via quoteSummary->result; wrapped.len={}",
                        i, wrapped.len()
                    );
                }
                return Ok(wrapped);
            }
        }

        if let Some(body_val) = val.get("body") {
            let payload_opt = match body_val {
                Value::String(s) => serde_json::from_str::<Value>(s).ok(),
                Value::Object(_) | Value::Array(_) => Some(body_val.clone()),
                _ => None,
            };

            if let Some(payload) = payload_opt {
                if let Some(qss) = find_quote_summary_store_in_value(&payload) {
                    let store_like = normalize_store_like(qss.clone());
                    let wrapped = wrap_store_like(store_like)?;
                    if debug {
                        eprintln!(
                            "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via body->QuoteSummaryStore; wrapped.len={}",
                            i, wrapped.len()
                        );
                    }
                    return Ok(wrapped);
                }

                if let Some(qs_val) = find_quote_summary_value_in_value(&payload) {
                    if let Some(store_like) = extract_store_like_from_quote_summary_value(qs_val) {
                        let wrapped = wrap_store_like(store_like)?;
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via body->quoteSummary->result; wrapped.len={}",
                                i, wrapped.len()
                            );
                        }
                        return Ok(wrapped);
                    }
                }
            }
        }
    }

    if debug {
        eprintln!("YF_DEBUG [extract_bootstrap_json]: All strategies exhausted; bootstrap not found.");
    }
    Err(YfError::Data("bootstrap not found".into()))
}

fn iter_json_scripts(html: &str) -> Vec<(&str, &str)> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");
    if debug {
        eprintln!(
            "YF_DEBUG [iter_json_scripts]: html.len()={}; scanning for <script> blocks...",
            html.len()
        );
    }

    let mut res = Vec::new();
    let mut pos = 0usize;
    let mut total_scripts = 0usize;
    let mut total_json_scripts = 0usize;
    let mut total_svelte_fetched = 0usize;

    while let Some(si) = html[pos..].find("<script") {
        let si = pos + si;
        total_scripts += 1;

        let open_end = match html[si..].find('>') {
            Some(x) => si + x,
            None => break,
        };
        let tag_open = &html[si..=open_end];

        let is_json = tag_open.contains("type=\"application/json\"");
        if is_json {
            total_json_scripts += 1;
            if tag_open.contains("data-sveltekit-fetched") {
                total_svelte_fetched += 1;
            }
        }

        let close = match html[open_end + 1..].find("</script>") {
            Some(x) => open_end + 1 + x,
            None => break,
        };
        let inner = &html[open_end + 1..close];

        if is_json {
            res.push((tag_open, inner));
        }
        pos = close + "</script>".len();
    }

    if debug {
        eprintln!(
            "YF_DEBUG [iter_json_scripts]: total_scripts={}, total_json_scripts={}, svelte_fetched={}",
            total_scripts, total_json_scripts, total_svelte_fetched
        );
        if let Some((attrs, body)) = res.get(0) {
            let a = if attrs.len() > 180 { &attrs[..180] } else { attrs };
            let b = if body.len() > 120 { &body[..120] } else { body };
            eprintln!(
                "YF_DEBUG [iter_json_scripts]: first JSON script attrs[trunc]=`{}` body[trunc]=`{}`",
                a, b
            );
        }
    }
    res
}

fn extract_store_like_from_quote_summary_value(qs_val: &Value) -> Option<Value> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");

    let summary = qs_val.get("quoteSummary").unwrap_or(qs_val);

    let result0 = summary
        .get("result")
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .cloned();

    if result0.is_none() {
        if debug {
            eprintln!(
                "YF_DEBUG [extract_store_like]: quoteSummary.result[0] missing or not an array."
            );
        }
        return None;
    }
    let result0 = result0.unwrap();

    let has_quote_type = result0.get("quoteType").is_some();
    let has_profile =
        result0.get("assetProfile").is_some() || result0.get("summaryProfile").is_some();
    let has_fund = result0.get("fundProfile").is_some();

    if debug {
        eprintln!(
            "YF_DEBUG [extract_store_like]: has_quoteType={}, has_profile={}, has_fund={}",
            has_quote_type, has_profile, has_fund
        );
    }
    if !has_quote_type && !(has_profile || has_fund) {
        if debug {
            eprintln!("YF_DEBUG [extract_store_like]: shape not acceptable.");
        }
        return None;
    }

    let norm = normalize_store_like(result0);
    if debug {
        let keys = norm
            .as_object()
            .map(|m| {
                let mut v: Vec<_> = m.keys().cloned().collect();
                v.sort();
                v.join(",")
            })
            .unwrap_or_default();
        eprintln!("YF_DEBUG [extract_store_like]: SUCCESS; normalized keys={}", keys);
    }
    Some(norm)
}

// Find an object that looks like a QuoteSummaryStore anywhere in a JSON tree.
fn find_quote_summary_store_in_value(v: &Value) -> Option<&Value> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");

    match v {
        Value::Object(map) => {
            if let Some(qss) = map.get("QuoteSummaryStore")
                && qss.is_object() {
                if debug {
                    eprintln!("YF_DEBUG [find_qss]: found direct 'QuoteSummaryStore' object.");
                }
                return Some(qss);
            }
            if let Some(stores) = map.get("stores")
                && let Some(qss) = stores.get("QuoteSummaryStore")
                    && qss.is_object() {
                if debug {
                    eprintln!("YF_DEBUG [find_qss]: found 'stores.QuoteSummaryStore' object.");
                }
                return Some(qss);
            }
            for child in map.values() {
                if let Some(found) = find_quote_summary_store_in_value(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => {
            for child in arr {
                if let Some(found) = find_quote_summary_store_in_value(child) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

// Locate a "quoteSummary" object anywhere in the JSON tree.
fn find_quote_summary_value_in_value(v: &Value) -> Option<&Value> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");

    match v {
        Value::Object(map) => {
            if let Some(qs) = map.get("quoteSummary") {
                if debug {
                    let got_res = qs.get("result").is_some();
                    let got_err = qs.get("error").is_some();
                    eprintln!(
                        "YF_DEBUG [find_qs]: found 'quoteSummary' (has result? {}, has error? {})",
                        got_res, got_err
                    );
                }
                return Some(qs);
            }
            for child in map.values() {
                if let Some(found) = find_quote_summary_value_in_value(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => {
            for child in arr {
                if let Some(found) = find_quote_summary_value_in_value(child) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn parse_jsonish_string(s: &str) -> Option<serde_json::Value> {
    let t = s.trim();
    if t.starts_with('{') || t.starts_with('[') {
        serde_json::from_str::<serde_json::Value>(t).ok()
    } else {
        None
    }
}

// Normalize: map assetProfile -> summaryProfile so downstream EQUITY path works uniformly.
fn normalize_store_like(mut store_like: Value) -> Value {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");
    if let Some(obj) = store_like.as_object_mut()
        && let Some(ap) = obj.remove("assetProfile") {
        if debug {
            eprintln!("YF_DEBUG [normalize_store_like]: moved assetProfile -> summaryProfile");
        }
        obj.insert("summaryProfile".to_string(), ap);
    }
    store_like
}

// Wrap the store-like object into the shape our Bootstrap deserializer expects.
fn wrap_store_like(store_like: Value) -> Result<String, YfError> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");
    let store_json = serde_json::to_string(&store_like)
        .map_err(|e| YfError::Data(format!("re-serialize: {e}")))?;
    if debug {
        let preview = if store_json.len() > 160 {
            format!("{} …[trunc]", &store_json[..160])
        } else {
            store_json.clone()
        };
        eprintln!(
            "YF_DEBUG [wrap_store_like]: wrapping store_like (len={}), preview=`{}`",
            store_json.len(),
            preview
        );
    }
    Ok(format!(
        r#"{{"context":{{"dispatcher":{{"stores":{{"QuoteSummaryStore":{store_json}}}}}}}}}"#
    ))
}

/// Find the index of the closing '}' matching the '{' at `start`.
/// String-aware (ignores braces inside JSON strings).
fn find_matching_brace(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let i = start;
    if bytes.get(i).copied()? != b'{' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_str = false;
    let mut j = i;

    while j < bytes.len() {
        let c = bytes[j];

        if in_str {
            if c == b'\\' {
                // skip escaped byte (next char is escaped)
                j += 2;
                continue;
            } else if c == b'"' {
                in_str = false;
            }
            j += 1;
            continue;
        }

        match c {
            b'"' => {
                in_str = true;
            }
            b'{' => {
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

impl Profile {
    /// Load a profile for `symbol` by scraping the Yahoo quote page bootstrap JSON.
    pub async fn load(client: &mut YfClient, symbol: &str) -> Result<Profile, YfError> {
        #[cfg(not(feature = "test-mode"))]
        {
            client.ensure_credentials().await?;

            match load_from_quote_summary_api(client, symbol).await {
                Ok(p) => return Ok(p),
                Err(e) => {
                    if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                        eprintln!("YF_DEBUG: API call failed ({e}), falling back to scrape.");
                    }
                }
            }

            load_from_scrape(client, symbol).await
        }

        #[cfg(feature = "test-mode")]
        {
            use crate::client::ApiPreference;
            match client.api_preference() {
                ApiPreference::ApiThenScrape => {
                    client.ensure_credentials().await?;
                    match load_from_quote_summary_api(client, symbol).await {
                        Ok(p) => return Ok(p),
                        Err(e) => {
                            if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                                eprintln!(
                                    "YF_DEBUG: API call failed ({e}), falling back to scrape."
                                );
                            }
                        }
                    }
                    load_from_scrape(client, symbol).await
                }
                ApiPreference::ApiOnly => {
                    client.ensure_credentials().await?;
                    load_from_quote_summary_api(client, symbol).await
                }
                ApiPreference::ScrapeOnly => load_from_scrape(client, symbol).await,
            }
        }
    }
}

use serde_json::Value;
use std::{fs, io::Write};

fn debug_dump_extracted_json(symbol: &str, json: &str) -> std::io::Result<()> {
    let path = std::env::temp_dir().join(format!("yfinance_rs-profile-{}-extracted.json", symbol));
    let mut f = std::fs::File::create(&path)?;

    // Try to pretty-print for readability
    if let Ok(val) = serde_json::from_str::<Value>(json)
        && let Ok(pretty) = serde_json::to_string_pretty(&val) {
            let _ = f.write_all(pretty.as_bytes());
            eprintln!(
                "YF_DEBUG: wrote pretty-printed extracted JSON to {}",
                path.display()
            );
            return Ok(());
        }

    // Fallback to raw string if pretty-printing fails
    let _ = f.write_all(json.as_bytes());
    eprintln!("YF_DEBUG: wrote raw extracted JSON to {}", path.display());
    Ok(())
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn debug_dump_html(symbol: &str, html: &str) -> std::io::Result<()> {
    let tmp = std::env::temp_dir();
    let base = format!("yfinance_rs-profile-{}", symbol);

    let min_path = tmp.join(format!("{}-min.html", base));
    let next_path = tmp.join(format!("{}-next.json", base));
    let rootapp_path = tmp.join(format!("{}-rootapp.json", base));

    // helper: safe-ish pretty print with char-limit
    fn pretty_limit(v: &Value, max_chars: usize) -> String {
        let s = serde_json::to_string_pretty(v).unwrap_or_else(|_| format!("{v:?}"));
        if s.chars().count() <= max_chars {
            return s;
        }
        let mut out = String::new();
        let mut n = 0usize;
        for ch in s.chars() {
            if n >= max_chars {
                break;
            }
            out.push(ch);
            n += 1;
        }
        out.push_str("\n… [truncated]");
        out
    }

    // helper: parse JSON if string contains JSON
    fn parse_jsonish_string(s: &str) -> Option<Value> {
        let t = s.trim();
        if t.starts_with('{') || t.starts_with('[') {
            serde_json::from_str::<Value>(t).ok()
        } else {
            None
        }
    }

    // helper: extract first <title>...</title>
    fn extract_title(html: &str) -> Option<String> {
        let lt = "<title>";
        let rt = "</title>";
        let i = html.find(lt)?;
        let j = html[i + lt.len()..].find(rt)? + i + lt.len();
        Some(html[i + lt.len()..j].to_string())
    }

    // helper: extract a JS object assigned after a pattern like `root.App.main = { ... }`
    fn extract_js_object_after(pattern: &str, s: &str) -> Option<String> {
        let start = s.find(pattern)? + pattern.len();
        let bytes = s.as_bytes();
        let mut i = start;
        // skip spaces
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'{' {
            return None;
        }
        let mut j = i;
        let mut depth = 0i32;
        while j < bytes.len() {
            match bytes[j] {
                b'{' => {
                    depth += 1;
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        j += 1;
                        break;
                    }
                }
                b'"' => {
                    // skip over a JSON string
                    j += 1;
                    while j < bytes.len() {
                        if bytes[j] == b'\\' {
                            j += 2;
                            continue;
                        }
                        if bytes[j] == b'"' {
                            j += 1;
                            break;
                        }
                        j += 1;
                    }
                    continue;
                }
                _ => {}
            }
            j += 1;
        }
        if j <= i {
            return None;
        }
        Some(s[i..j].to_string())
    }

    // Start building the minimal HTML
    let mut min_html = String::new();
    min_html.push_str("<!doctype html><meta charset=\"utf-8\">\n<style>pre{white-space:pre-wrap;font:12px/1.3 ui-monospace,monospace}</style>\n");
    min_html.push_str(&format!("<!-- compact debug for {} -->\n", symbol));
    if let Some(t) = extract_title(html) {
        min_html.push_str(&format!("<h1>title</h1><pre>{}</pre>\n", escape_html(&t)));
    }

    // 1) SvelteKit & other JSON scripts
    let mut svelte_count = 0usize;
    for (attrs, inner) in iter_json_scripts(html) {
        // try to parse this script content as JSON
        let parsed = serde_json::from_str::<Value>(inner).ok();

        // SvelteKit style has `data-sveltekit-fetched` with a `"body"` that is itself JSON text
        let mut pretty = String::new();
        if let Some(v) = parsed.as_ref() {
            if let Some(u) = v
                .get("body")
                .and_then(|b| b.as_str())
                .and_then(parse_jsonish_string)
            {
                pretty = pretty_limit(&u, 5_000);
                svelte_count += 1;
            } else {
                pretty = pretty_limit(v, 5_000);
            }
        } else if let Some(v) = parse_jsonish_string(inner) {
            pretty = pretty_limit(&v, 5_000);
        } else {
            continue; // skip junk
        }

        // show a small header with key attrs (data-url if present)
        let data_url = attrs
            .split_whitespace()
            .find(|p| p.starts_with("data-url="))
            .map(|p| p.trim_start_matches("data-url=").trim_matches('"'))
            .unwrap_or("");
        let label = if attrs.contains("data-sveltekit-fetched") {
            format!("sveltekit-fetched {}", data_url)
        } else if attrs.contains("id=\"__NEXT_DATA__\"") {
            "__NEXT_DATA__".to_string()
        } else {
            "application/json script".to_string()
        };
        min_html.push_str(&format!(
            "<h2>{}</h2><pre>{}</pre>\n",
            escape_html(&label),
            escape_html(&pretty)
        ));
    }

    // 2) Extract __NEXT_DATA__ as a dedicated JSON file (if present)
    if let Some((_, inner)) = iter_json_scripts(html)
        .into_iter()
        .find(|(attrs, _)| attrs.contains("id=\"__NEXT_DATA__\""))
        && let Ok(v) = serde_json::from_str::<Value>(inner) {
            let mut f = fs::File::create(&next_path)?;
            let s = serde_json::to_string_pretty(&v).unwrap_or_else(|_| inner.to_string());
            f.write_all(s.as_bytes())?;
            eprintln!("YF_DEBUG: wrote {}", next_path.display());
        }

    // 3) Extract legacy root.App.main JSON (some regions still serve it)
    if let Some(js_obj) = extract_js_object_after("root.App.main =", html) {
        if let Ok(v) = serde_json::from_str::<Value>(&js_obj) {
            let mut f = fs::File::create(&rootapp_path)?;
            let s = serde_json::to_string_pretty(&v).unwrap_or(js_obj.clone());
            f.write_all(s.as_bytes())?;
            eprintln!("YF_DEBUG: wrote {}", rootapp_path.display());
        }
        // also show a short snippet inside the min html
        if let Ok(v) = serde_json::from_str::<Value>(&js_obj) {
            min_html.push_str("<h2>root.App.main (snippet)</h2>\n");
            let pretty = pretty_limit(&v, 5_000);
            min_html.push_str(&format!("<pre>{}</pre>\n", escape_html(&pretty)));
        }
    }

    // Finally write the compact HTML
    let mut f = fs::File::create(&min_path)?;
    f.write_all(min_html.as_bytes())?;
    eprintln!("YF_DEBUG: wrote {}", min_path.display());

    Ok(())
}

fn debug_dump_api(symbol: &str, body: &str) -> std::io::Result<()> {
    use std::io::Write;
    let path = std::env::temp_dir().join(format!("yfinance_rs-quoteSummary-{}.json", symbol));
    let mut f = std::fs::File::create(&path)?;
    let _ = f.write_all(body.as_bytes());
    eprintln!("YF_DEBUG=1: wrote {}", path.display());
    Ok(())
}

async fn load_from_quote_summary_api(
    client: &mut YfClient,
    symbol: &str,
) -> Result<Profile, YfError> {
    for i in 0..=1 {
        let crumb = client
            .crumb()
            .ok_or(YfError::Data("Crumb is not set".into()))?;
        let mut url = client.base_quote_api().join(symbol)?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("modules", "assetProfile,quoteType,fundProfile");
            qp.append_pair("crumb", crumb);
        }

        let resp = client.http().get(url.clone()).send().await?;
        let text = crate::net::get_text(resp, "profile_api", symbol, "json").await?;

        let env: V10Envelope = serde_json::from_str(&text)
            .map_err(|e| YfError::Data(format!("quoteSummary json parse: {e}")))?;

        if let Some(error) = env.quote_summary.as_ref().and_then(|qs| qs.error.as_ref()) {
            if error.description.contains("Invalid Crumb") && i == 0 {
                if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                    eprintln!(
                        "YF_DEBUG: Invalid crumb detected. Refreshing credentials and retrying."
                    );
                }
                client.clear_crumb();
                client.ensure_credentials().await?;
                continue;
            }
            return Err(YfError::Data(format!("yahoo error: {}", error.description)));
        }

        let first = env
            .quote_summary
            .and_then(|qs| qs.result)
            .and_then(|mut v| v.pop())
            .ok_or_else(|| YfError::Data("empty quoteSummary result".into()))?;

        let kind = first
            .quote_type
            .as_ref()
            .and_then(|q| q.quote_type.as_deref())
            .unwrap_or("");
        let name = first
            .quote_type
            .as_ref()
            .and_then(|q| q.long_name.clone().or(q.short_name.clone()))
            .unwrap_or_else(|| symbol.to_string());

        return match kind {
            "EQUITY" => {
                let sp = first
                    .asset_profile
                    .ok_or_else(|| YfError::Data("assetProfile missing".into()))?;
                let address = Address {
                    street1: sp.address1,
                    street2: sp.address2,
                    city: sp.city,
                    state: sp.state,
                    country: sp.country,
                    zip: sp.zip,
                };
                Ok(Profile::Company(Company {
                    name,
                    sector: sp.sector,
                    industry: sp.industry,
                    website: sp.website,
                    summary: sp.long_business_summary,
                    address: Some(address),
                }))
            }
            "ETF" => {
                let fp = first
                    .fund_profile
                    .ok_or_else(|| YfError::Data("fundProfile missing".into()))?;
                Ok(Profile::Fund(Fund {
                    name,
                    family: fp.family,
                    kind: fp.legal_type.unwrap_or_else(|| "Fund".to_string()),
                }))
            }
            other => Err(YfError::Data(format!("unsupported quoteType: {other}"))),
        };
    }

    Err(YfError::Data("API call failed after retry".into()))
}

/* --------- Minimal serde mapping for the bootstrap JSON --------- */

#[derive(Deserialize)]
struct Bootstrap {
    context: Ctx,
}

#[derive(Deserialize)]
struct Ctx {
    dispatcher: Dispatch,
}

#[derive(Deserialize)]
struct Dispatch {
    stores: Stores,
}

#[derive(Deserialize)]
struct Stores {
    #[serde(rename = "QuoteSummaryStore")]
    quote_summary_store: QuoteSummaryStore,
}

#[derive(Deserialize)]
struct QuoteSummaryStore {
    // CHANGED: make quoteType optional (Yahoo pages sometimes omit it in this blob).
    #[serde(rename = "quoteType")]
    quote_type: Option<QuoteTypeNode>,

    // CHANGED: include price node (for fallback longName/shortName).
    #[serde(default)]
    price: Option<PriceNode>,

    #[serde(rename = "summaryProfile")]
    summary_profile: Option<SummaryProfileNode>,

    #[serde(rename = "fundProfile")]
    fund_profile: Option<FundProfileNode>,
}

#[derive(Deserialize)]
struct QuoteTypeNode {
    #[serde(rename = "quoteType")]
    kind: Option<String>,

    #[serde(rename = "longName")]
    long_name: Option<String>,

    #[serde(rename = "shortName")]
    short_name: Option<String>,
}

#[derive(Deserialize)]
struct PriceNode {
    #[serde(rename = "longName")]
    long_name: Option<String>,
    #[serde(rename = "shortName")]
    short_name: Option<String>,
}

#[derive(Deserialize)]
struct SummaryProfileNode {
    address1: Option<String>,
    address2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    country: Option<String>,
    zip: Option<String>,
    sector: Option<String>,
    industry: Option<String>,

    #[serde(rename = "longBusinessSummary")]
    long_business_summary: Option<String>,

    website: Option<String>,
}

#[derive(Deserialize)]
struct FundProfileNode {
    #[serde(rename = "legalType")]
    legal_type: Option<String>,
    family: Option<String>,
}

#[derive(serde::Deserialize)]
struct V10Envelope {
    #[serde(rename = "quoteSummary")]
    quote_summary: Option<V10QuoteSummary>,
}

#[derive(serde::Deserialize)]
struct V10QuoteSummary {
    result: Option<Vec<V10Result>>,
    error: Option<V10Error>,
}

#[derive(serde::Deserialize)]
struct V10Error {
    description: String,
}

#[derive(serde::Deserialize)]
struct V10Result {
    #[serde(rename = "assetProfile")]
    asset_profile: Option<V10AssetProfile>,
    #[serde(rename = "fundProfile")]
    fund_profile: Option<V10FundProfile>,
    #[serde(rename = "quoteType")]
    quote_type: Option<V10QuoteType>,
}

#[derive(serde::Deserialize)]
struct V10AssetProfile {
    address1: Option<String>,
    address2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    country: Option<String>,
    zip: Option<String>,
    sector: Option<String>,
    industry: Option<String>,
    website: Option<String>,
    #[serde(rename = "longBusinessSummary")]
    long_business_summary: Option<String>,
}

#[derive(serde::Deserialize)]
struct V10FundProfile {
    #[serde(rename = "legalType")]
    legal_type: Option<String>,
    family: Option<String>,
}

#[derive(serde::Deserialize)]
struct V10QuoteType {
    #[serde(rename = "quoteType")]
    quote_type: Option<String>,
    #[serde(rename = "longName")]
    long_name: Option<String>,
    #[serde(rename = "shortName")]
    short_name: Option<String>,
}
