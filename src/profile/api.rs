//! quoteSummary v10 API path for profiles.

use crate::{YfClient, YfError, core::net};
use serde::Deserialize;

use super::{Address, Company, Fund, Profile};

#[cfg(any(debug_assertions, feature = "debug-dumps"))]
use crate::profile::debug::debug_dump_api;

pub(crate) async fn load_from_quote_summary_api(
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
        let text = net::get_text(resp, "profile_api", symbol, "json").await?;

        #[cfg(any(debug_assertions, feature = "debug-dumps"))]
        {
            let _ = debug_dump_api(symbol, &text);
        }

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

/* --------- Minimal serde mapping for the API JSON --------- */

#[derive(Deserialize)]
struct V10Envelope {
    #[serde(rename = "quoteSummary")]
    quote_summary: Option<V10QuoteSummary>,
}

#[derive(Deserialize)]
struct V10QuoteSummary {
    result: Option<Vec<V10Result>>,
    error: Option<V10Error>,
}

#[derive(Deserialize)]
struct V10Error {
    description: String,
}

#[derive(Deserialize)]
struct V10Result {
    #[serde(rename = "assetProfile")]
    asset_profile: Option<V10AssetProfile>,
    #[serde(rename = "fundProfile")]
    fund_profile: Option<V10FundProfile>,
    #[serde(rename = "quoteType")]
    quote_type: Option<V10QuoteType>,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
struct V10FundProfile {
    #[serde(rename = "legalType")]
    legal_type: Option<String>,
    family: Option<String>,
}

#[derive(Deserialize)]
struct V10QuoteType {
    #[serde(rename = "quoteType")]
    quote_type: Option<String>,
    #[serde(rename = "longName")]
    long_name: Option<String>,
    #[serde(rename = "shortName")]
    short_name: Option<String>,
}
