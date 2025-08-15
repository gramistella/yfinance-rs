pub(crate) fn iter_json_scripts(html: &str) -> Vec<(&str, &str)> {
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

/// Exposed for debug helpers as well.
pub(crate) fn find_matching_brace(s: &str, start: usize) -> Option<usize> {
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

/// Exposed for debug helpers as well.
pub(crate) fn parse_jsonish_string(s: &str) -> Option<serde_json::Value> {
    let t = s.trim();
    if t.starts_with('{') || t.starts_with('[') {
        serde_json::from_str::<serde_json::Value>(t).ok()
    } else {
        None
    }
}

pub fn escape_html(s: &str) -> String {
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