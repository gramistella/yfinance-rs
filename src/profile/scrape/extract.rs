//! HTML extraction helpers for QuoteSummaryStore / quoteSummary payloads.

use crate::profile::scrape::utils::{find_matching_brace, iter_json_scripts};
use serde_json::Value;

/// Pulls a bootstrapped JSON blob from the Yahoo quote HTML.
/// Returns a *wrapped* JSON string with a `QuoteSummaryStore` under:
/// `{"context":{"dispatcher":{"stores":{"QuoteSummaryStore":{...}}}}}`
pub(crate) fn extract_bootstrap_json(body: &str) -> Result<String, crate::YfError> {
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
                eprintln!(
                    "YF_DEBUG [extract_bootstrap_json]: Strategy B found start but failed to match closing brace."
                );
            }
        }
    }

    /* Strategy C: SvelteKit data-sveltekit-fetched blobs. */
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
                        eprintln!(
                            "YF_DEBUG [extract_bootstrap_json]: C[{}][{}] nodes.len={}",
                            i,
                            ai,
                            nodes.len()
                        );
                    }
                    for (ni, node) in nodes.iter().enumerate() {
                        if let Some(data) = node.get("data")
                            && let Some(store_like) =
                                extract_store_like_from_quote_summary_value(data)
                        {
                            let wrapped = wrap_store_like(store_like)?;
                            if debug {
                                eprintln!(
                                    "YF_DEBUG [extract_bootstrap_json]: C[{}][{}] SUCCESS via nodes[{}].data -> wrapped.len={}",
                                    i,
                                    ai,
                                    ni,
                                    wrapped.len()
                                );
                            }
                            return Ok(wrapped);
                        }
                    }
                }
            }
        }

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
            let body_val_opt = { outer_obj.get_mut("body").map(|b| b.take()) };

            if let Some(body_val) = body_val_opt {
                let payload_opt = match body_val {
                    Value::String(s) => serde_json::from_str::<Value>(&s).ok(),
                    Value::Object(_) | Value::Array(_) => Some(body_val),
                    _ => None,
                };

                if let Some(payload) = payload_opt {
                    if let Some(qss) = find_quote_summary_store_in_value(&payload) {
                        let store_like = normalize_store_like(qss.clone());
                        let wrapped = wrap_store_like(store_like)?;
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: C[{}] SUCCESS via QuoteSummaryStore path; wrapped.len={}",
                                i,
                                wrapped.len()
                            );
                        }
                        return Ok(wrapped);
                    }

                    if let Some(qs_val) = find_quote_summary_value_in_value(&payload)
                        && let Some(store_like) =
                            extract_store_like_from_quote_summary_value(qs_val)
                    {
                        let wrapped = wrap_store_like(store_like)?;
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: C[{}] SUCCESS via quoteSummary->result; wrapped.len={}",
                                i,
                                wrapped.len()
                            );
                        }
                        return Ok(wrapped);
                    }
                }
            }
        }
    }

    /* Strategy D: scan ALL application/json scripts generically */
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
                        if inner_json.len() > 120 {
                            &inner_json[..120]
                        } else {
                            inner_json
                        }
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
                    i,
                    wrapped.len()
                );
            }
            return Ok(wrapped);
        }

        if let Some(qs_val) = find_quote_summary_value_in_value(&val)
            && let Some(store_like) = extract_store_like_from_quote_summary_value(qs_val)
        {
            let wrapped = wrap_store_like(store_like)?;
            if debug {
                eprintln!(
                    "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via quoteSummary->result; wrapped.len={}",
                    i,
                    wrapped.len()
                );
            }
            return Ok(wrapped);
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
                            i,
                            wrapped.len()
                        );
                    }
                    return Ok(wrapped);
                }

                if let Some(qs_val) = find_quote_summary_value_in_value(&payload)
                    && let Some(store_like) = extract_store_like_from_quote_summary_value(qs_val)
                {
                    let wrapped = wrap_store_like(store_like)?;
                    if debug {
                        eprintln!(
                            "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via body->quoteSummary->result; wrapped.len={}",
                            i,
                            wrapped.len()
                        );
                    }
                    return Ok(wrapped);
                }
            }
        }
    }

    if debug {
        eprintln!(
            "YF_DEBUG [extract_bootstrap_json]: All strategies exhausted; bootstrap not found."
        );
    }
    Err(crate::YfError::Data("bootstrap not found".into()))
}

/* ---------------------- internal utilities ---------------------- */

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
    if !(has_quote_type || has_profile || has_fund) {
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
        eprintln!(
            "YF_DEBUG [extract_store_like]: SUCCESS; normalized keys={}",
            keys
        );
    }
    Some(norm)
}

fn find_quote_summary_store_in_value(v: &Value) -> Option<&Value> {
    match v {
        Value::Object(map) => {
            if let Some(qss) = map.get("QuoteSummaryStore")
                && qss.is_object()
            {
                return Some(qss);
            }
            if let Some(stores) = map.get("stores")
                && let Some(qss) = stores.get("QuoteSummaryStore")
                && qss.is_object()
            {
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

fn find_quote_summary_value_in_value(v: &Value) -> Option<&Value> {
    match v {
        Value::Object(map) => {
            if let Some(qs) = map.get("quoteSummary") {
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

fn normalize_store_like(mut store_like: Value) -> Value {
    if let Some(obj) = store_like.as_object_mut()
        && let Some(ap) = obj.remove("assetProfile")
    {
        obj.insert("summaryProfile".to_string(), ap);
    }
    store_like
}

fn wrap_store_like(store_like: Value) -> Result<String, crate::YfError> {
    let store_json = serde_json::to_string(&store_like)
        .map_err(|e| crate::YfError::Data(format!("re-serialize: {e}")))?;
    Ok(format!(
        r#"{{"context":{{"dispatcher":{{"stores":{{"QuoteSummaryStore":{store_json}}}}}}}}}"#
    ))
}
