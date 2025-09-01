mod helpers;
mod strategies;

use helpers::truncate;
use strategies::{
    try_generic_json_scripts, try_quote_summary_store_literal, try_root_app_main,
    try_sveltekit_json,
};

pub fn extract_bootstrap_json(body: &str) -> Result<String, crate::YfError> {
    let debug = std::env::var("YF_DEBUG").ok().as_deref() == Some("1");

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
    if let Some(json_str) = try_root_app_main(body, debug) {
        if debug {
            eprintln!(
                "YF_DEBUG [extract_bootstrap_json]: Strategy A hit; json.len={} preview=`{}`",
                json_str.len(),
                truncate(&json_str, 160)
            );
        }
        return Ok(json_str);
    }

    /* Strategy B: literal "QuoteSummaryStore": { ... } object */
    if debug {
        eprintln!("YF_DEBUG [extract_bootstrap_json]: Strategy B (QuoteSummaryStore literal)...");
    }
    if let Some(wrapped) = try_quote_summary_store_literal(body, debug) {
        if debug {
            eprintln!(
                "YF_DEBUG [extract_bootstrap_json]: Strategy B hit; wrapped.len={} preview=`{}`",
                wrapped.len(),
                truncate(&wrapped, 160)
            );
        }
        return Ok(wrapped);
    }

    /* Strategy C: SvelteKit data-sveltekit-fetched blobs. */
    if debug {
        eprintln!("YF_DEBUG [extract_bootstrap_json]: Strategy C (SvelteKit fetched JSON)...");
    }
    if let Some(wrapped) = try_sveltekit_json(body, debug) {
        if debug {
            eprintln!(
                "YF_DEBUG [extract_bootstrap_json]: Strategy C hit; wrapped.len={} preview=`{}`",
                wrapped.len(),
                truncate(&wrapped, 160)
            );
        }
        return Ok(wrapped);
    }

    /* Strategy D: generic scan of all application/json scripts */
    if debug {
        eprintln!("YF_DEBUG [extract_bootstrap_json]: Strategy D (generic JSON scan)...");
    }
    if let Some(wrapped) = try_generic_json_scripts(body, debug) {
        if debug {
            eprintln!(
                "YF_DEBUG [extract_bootstrap_json]: Strategy D hit; wrapped.len={} preview=`{}`",
                wrapped.len(),
                truncate(&wrapped, 160)
            );
        }
        return Ok(wrapped);
    }

    if debug {
        eprintln!(
            "YF_DEBUG [extract_bootstrap_json]: All strategies exhausted; bootstrap not found."
        );
    }
    Err(crate::YfError::MissingData("bootstrap not found".into()))
}
