//! quoteSummary v10 API path for profiles.

use crate::{
    YfClient, YfError,
    core::{client::CacheMode, conversions::string_to_fund_kind, quotesummary},
};
use serde::Deserialize;

use super::{Address, Company, Fund, Profile};

pub async fn load_from_quote_summary_api(
    client: &YfClient,
    symbol: &str,
) -> Result<Profile, YfError> {
    let first: V10Result = quotesummary::fetch_module_result(
        client,
        symbol,
        "assetProfile,quoteType,fundProfile",
        "profile",
        CacheMode::Use,
        None,
    )
    .await?;

    let kind = first
        .quote_type
        .as_ref()
        .and_then(|q| q.quote_type.as_deref())
        .unwrap_or("");

    let name = first
        .quote_type
        .as_ref()
        .and_then(|q| q.long_name.clone().or_else(|| q.short_name.clone()))
        .unwrap_or_else(|| symbol.to_string());

    match kind {
        "EQUITY" => {
            let sp = first
                .asset_profile
                .ok_or_else(|| YfError::MissingData("assetProfile missing".into()))?;
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
                isin: sp.isin,
            }))
        }
        "ETF" => {
            let fp = first
                .fund_profile
                .ok_or_else(|| YfError::MissingData("fundProfile missing".into()))?;
            Ok(Profile::Fund(Fund {
                name,
                family: fp.family,
                kind: string_to_fund_kind(fp.legal_type).unwrap_or_default(),
                isin: fp.isin,
            }))
        }
        other => Err(YfError::InvalidParams(format!(
            "unsupported quoteType: {other}"
        ))),
    }
}

/* --------- Minimal serde mapping for the API JSON --------- */

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
    isin: Option<String>,
}

#[derive(Deserialize)]
struct V10FundProfile {
    #[serde(rename = "legalType")]
    legal_type: Option<String>,
    family: Option<String>,
    isin: Option<String>,
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
