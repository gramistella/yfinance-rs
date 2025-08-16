//! Scrape the Yahoo quote HTML and extract profile data.

use crate::{YfClient, YfError};
use serde::Deserialize;

use super::{Address, Company, Fund, Profile};

#[cfg(any(debug_assertions, feature = "debug-dumps"))]
use crate::profile::debug::{debug_dump_extracted_json, debug_dump_html};

pub(crate) mod extract;
pub(crate) mod utils;
use extract::extract_bootstrap_json;

pub(crate) async fn load_from_scrape(client: &YfClient, symbol: &str) -> Result<Profile, YfError> {
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
    let body = crate::core::net::get_text(quote_page_resp, "profile_html", symbol, "html").await?;

    #[cfg(any(debug_assertions, feature = "debug-dumps"))]
    {
        if debug {
            let _ = debug_dump_html(symbol, &body);
        }
    }

    let json_str = extract_bootstrap_json(&body)?;
    #[cfg(any(debug_assertions, feature = "debug-dumps"))]
    {
        if debug {
            let _ = debug_dump_extracted_json(symbol, &json_str);
        }
    }

    let boot: Bootstrap = serde_json::from_str(&json_str)
        .map_err(|e| YfError::Data(format!("bootstrap json parse: {e}")))?;

    let store = boot.context.dispatcher.stores.quote_summary_store;

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
    #[serde(rename = "quoteType")]
    quote_type: Option<QuoteTypeNode>,

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
