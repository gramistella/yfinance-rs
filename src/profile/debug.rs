//! Debug dump helpers for development / troubleshooting.

use serde_json::Value;
use std::io::Write;
use crate::profile::scrape::utils::{iter_json_scripts, parse_jsonish_string, escape_html};

pub fn debug_dump_extracted_json(symbol: &str, json: &str) -> std::io::Result<()> {
    let path = std::env::temp_dir().join(format!("yfinance_rs-profile-{}-extracted.json", symbol));
    let mut f = std::fs::File::create(&path)?;

    if let Ok(val) = serde_json::from_str::<Value>(json)
        && let Ok(pretty) = serde_json::to_string_pretty(&val) {
            let _ = f.write_all(pretty.as_bytes());
            eprintln!(
                "YF_DEBUG: wrote pretty-printed extracted JSON to {}",
                path.display()
            );
            return Ok(());
        }

    let _ = f.write_all(json.as_bytes());
    eprintln!("YF_DEBUG: wrote raw extracted JSON to {}", path.display());
    Ok(())
}

pub fn debug_dump_html(symbol: &str, html: &str) -> std::io::Result<()> {
    use std::{fs, io::Write};

    let tmp = std::env::temp_dir();
    let base = format!("yfinance_rs-profile-{}", symbol);

    let min_path = tmp.join(format!("{}-min.html", base));
    let next_path = tmp.join(format!("{}-next.json", base));
    let rootapp_path = tmp.join(format!("{}-rootapp.json", base));

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

    fn extract_title(html: &str) -> Option<String> {
        let lt = "<title>";
        let rt = "</title>";
        let i = html.find(lt)?;
        let j = html[i + lt.len()..].find(rt)? + i + lt.len();
        Some(html[i + lt.len()..j].to_string())
    }

    fn extract_js_object_after(pattern: &str, s: &str) -> Option<String> {
        let start = s.find(pattern)? + pattern.len();
        let bytes = s.as_bytes();
        let mut i = start;
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

    let mut min_html = String::new();
    min_html.push_str("<!doctype html><meta charset=\"utf-8\">\n<style>pre{white-space:pre-wrap;font:12px/1.3 ui-monospace,monospace}</style>\n");
    min_html.push_str(&format!("<!-- compact debug for {} -->\n", symbol));
    if let Some(t) = extract_title(html) {
        min_html.push_str(&format!("<h1>title</h1><pre>{}</pre>\n", escape_html(&t)));
    }

    let mut svelte_count = 0usize;
    for (attrs, inner) in iter_json_scripts(html) {
        let parsed = serde_json::from_str::<Value>(inner).ok();

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
            continue;
        }

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

    if let Some((_, inner)) = iter_json_scripts(html)
        .into_iter()
        .find(|(attrs, _)| attrs.contains("id=\"__NEXT_DATA__\""))
        && let Ok(v) = serde_json::from_str::<Value>(inner) {
            let mut f = fs::File::create(&next_path)?;
            let s = serde_json::to_string_pretty(&v).unwrap_or_else(|_| inner.to_string());
            f.write_all(s.as_bytes())?;
            eprintln!("YF_DEBUG: wrote {}", next_path.display());
        }

    if let Some(js_obj) = extract_js_object_after("root.App.main =", html) {
        if let Ok(v) = serde_json::from_str::<Value>(&js_obj) {
            let mut f = fs::File::create(&rootapp_path)?;
            let s = serde_json::to_string_pretty(&v).unwrap_or(js_obj.clone());
            f.write_all(s.as_bytes())?;
            eprintln!("YF_DEBUG: wrote {}", rootapp_path.display());
        }
        if let Ok(v) = serde_json::from_str::<Value>(&js_obj) {
            min_html.push_str("<h2>root.App.main (snippet)</h2>\n");
            let pretty = pretty_limit(&v, 5_000);
            min_html.push_str(&format!("<pre>{}</pre>\n", escape_html(&pretty)));
        }
    }

    let mut f = std::fs::File::create(&min_path)?;
    f.write_all(min_html.as_bytes())?;
    eprintln!("YF_DEBUG: wrote {}", min_path.display());

    Ok(())
}

pub fn debug_dump_api(symbol: &str, body: &str) -> std::io::Result<()> {
    use std::io::Write;
    let path = std::env::temp_dir().join(format!("yfinance_rs-quoteSummary-{}.json", symbol));
    let mut f = std::fs::File::create(&path)?;
    let _ = f.write_all(body.as_bytes());
    eprintln!("YF_DEBUG=1: wrote {}", path.display());
    Ok(())
}
