use serde_json::Value;

pub fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        let mut out = String::with_capacity(n + 16);
        out.push_str(&s[..n]);
        out.push_str(" …[trunc]");
        out
    }
}

pub fn extract_store_like_from_quote_summary_value(qs_val: &Value) -> Option<Value> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");

    // Accept either {..., quoteSummary: {...}} or a quoteSummary node directly.
    let summary = qs_val.get("quoteSummary").unwrap_or(qs_val);

    // Take the first result element if present.
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

    // Sanity checks that this looks like a profile payload.
    let has_quote_type = result0.get("quoteType").is_some();
    let has_profile =
        result0.get("assetProfile").is_some() || result0.get("summaryProfile").is_some();
    let has_fund = result0.get("fundProfile").is_some();

    if debug {
        eprintln!(
            "YF_DEBUG [extract_store_like]: has_quoteType={has_quote_type}, has_profile={has_profile}, has_fund={has_fund}"
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
        eprintln!("YF_DEBUG [extract_store_like]: SUCCESS; normalized keys={keys}");
    }
    Some(norm)
}

pub fn find_quote_summary_store_in_value(v: &Value) -> Option<&Value> {
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

pub fn find_quote_summary_value_in_value(v: &Value) -> Option<&Value> {
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

pub fn normalize_store_like(mut store_like: Value) -> Value {
    if let Some(obj) = store_like.as_object_mut()
        && let Some(ap) = obj.remove("assetProfile")
    {
        // Normalize to what the rest of the code expects.
        obj.insert("summaryProfile".to_string(), ap);
    }
    store_like
}

pub fn wrap_store_like(store_like: &Value) -> Result<String, crate::YfError> {
    let store_json = serde_json::to_string(&store_like).map_err(|e| crate::YfError::Json(e))?;
    Ok(format!(
        r#"{{"context":{{"dispatcher":{{"stores":{{"QuoteSummaryStore":{store_json}}}}}}}}}"#
    ))
}
