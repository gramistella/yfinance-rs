use serde_json::Value;

use super::helpers::{
    extract_store_like_from_quote_summary_value, find_quote_summary_store_in_value,
    find_quote_summary_value_in_value, normalize_store_like, truncate, wrap_store_like,
};
use crate::profile::scrape::utils::{find_matching_brace, iter_json_scripts};

/// Strategy A: look for `root.App.main = {...};`
pub(crate) fn try_root_app_main(body: &str, debug: bool) -> Option<String> {
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
                        "YF_DEBUG [extract_bootstrap_json]: Strategy A preview=`{}`",
                        truncate(json_str, 160)
                    );
                }
                return Some(json_str.to_string());
            }
        }
    }
    None
}

/// Strategy B: find literal `"QuoteSummaryStore" : { ... }` and wrap it.
pub(crate) fn try_quote_summary_store_literal(body: &str, debug: bool) -> Option<String> {
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
                        "YF_DEBUG [extract_bootstrap_json]: Strategy B obj.len={} preview=`{}`",
                        obj.len(),
                        truncate(obj, 160)
                    );
                }
                return Some(wrapped);
            } else if debug {
                eprintln!(
                    "YF_DEBUG [extract_bootstrap_json]: Strategy B found start but failed to match closing brace."
                );
            }
        }
    }
    None
}

/// Strategy C: scan SvelteKit `data-sveltekit-fetched` JSON blobs.
pub(crate) fn try_sveltekit_json(body: &str, debug: bool) -> Option<String> {
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
                truncate(tag_attrs, 160),
                inner_json.len(),
                truncate(inner_json, 120)
            );
        }

        // Case C1: array of objects having nodes[].data
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
                            && let Ok(wrapped) = wrap_store_like(store_like)
                        {
                            if debug {
                                eprintln!(
                                    "YF_DEBUG [extract_bootstrap_json]: C[{}][{}] SUCCESS via nodes[{}].data -> wrapped.len={}",
                                    i,
                                    ai,
                                    ni,
                                    wrapped.len()
                                );
                            }
                            return Some(wrapped);
                        }
                    }
                }
            }
        }

        // Case C2: object with "body" either JSON string or inline JSON
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
                        if let Ok(wrapped) = wrap_store_like(store_like) {
                            if debug {
                                eprintln!(
                                    "YF_DEBUG [extract_bootstrap_json]: C[{}] SUCCESS via QuoteSummaryStore path; wrapped.len={}",
                                    i,
                                    wrapped.len()
                                );
                            }
                            return Some(wrapped);
                        }
                    }

                    if let Some(qs_val) = find_quote_summary_value_in_value(&payload)
                        && let Some(store_like) =
                            extract_store_like_from_quote_summary_value(qs_val)
                        && let Ok(wrapped) = wrap_store_like(store_like)
                    {
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: C[{}] SUCCESS via quoteSummary->result; wrapped.len={}",
                                i,
                                wrapped.len()
                            );
                        }
                        return Some(wrapped);
                    }
                }
            }
        }
    }

    None
}

/// Strategy D: generic scan of *all* application/json scripts with multiple fallbacks.
pub(crate) fn try_generic_json_scripts(body: &str, debug: bool) -> Option<String> {
    let scripts = iter_json_scripts(body);

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

        // D1: direct QuoteSummaryStore object
        if let Some(qss) = find_quote_summary_store_in_value(&val) {
            let store_like = normalize_store_like(qss.clone());
            if let Ok(wrapped) = wrap_store_like(store_like) {
                if debug {
                    eprintln!(
                        "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via QuoteSummaryStore; wrapped.len={}",
                        i,
                        wrapped.len()
                    );
                }
                return Some(wrapped);
            }
        }

        // D2: quoteSummary -> result[0]
        if let Some(qs_val) = find_quote_summary_value_in_value(&val)
            && let Some(store_like) = extract_store_like_from_quote_summary_value(qs_val)
            && let Ok(wrapped) = wrap_store_like(store_like)
        {
            if debug {
                eprintln!(
                    "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via quoteSummary->result; wrapped.len={}",
                    i,
                    wrapped.len()
                );
            }
            return Some(wrapped);
        }

        // D3: value has a "body" which itself is a JSON string/object/array
        if let Some(body_val) = val.get("body") {
            let payload_opt = match body_val {
                Value::String(s) => serde_json::from_str::<Value>(s).ok(),
                Value::Object(_) | Value::Array(_) => Some(body_val.clone()),
                _ => None,
            };

            if let Some(payload) = payload_opt {
                if let Some(qss) = find_quote_summary_store_in_value(&payload) {
                    let store_like = normalize_store_like(qss.clone());
                    if let Ok(wrapped) = wrap_store_like(store_like) {
                        if debug {
                            eprintln!(
                                "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via body->QuoteSummaryStore; wrapped.len={}",
                                i,
                                wrapped.len()
                            );
                        }
                        return Some(wrapped);
                    }
                }

                if let Some(qs_val) = find_quote_summary_value_in_value(&payload)
                    && let Some(store_like) = extract_store_like_from_quote_summary_value(qs_val)
                    && let Ok(wrapped) = wrap_store_like(store_like)
                {
                    if debug {
                        eprintln!(
                            "YF_DEBUG [extract_bootstrap_json]: D[{}] SUCCESS via body->quoteSummary->result; wrapped.len={}",
                            i,
                            wrapped.len()
                        );
                    }
                    return Some(wrapped);
                }
            }
        }
    }

    None
}
