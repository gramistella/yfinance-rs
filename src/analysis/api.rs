use serde::Deserialize;

use crate::analysis::model::PriceTarget;
use crate::core::net;
use crate::core::{YfClient, YfError};

#[cfg(any(debug_assertions, feature = "debug-dumps"))]
use crate::profile::debug::debug_dump_api;

/* ---------- Public entry points ---------- */

pub async fn recommendation_trend(
    client: &mut YfClient,
    symbol: &str,
) -> Result<Vec<super::RecommendationRow>, YfError> {
    let env = call_quote_summary(client, symbol, "recommendationTrend").await?;
    let root = get_first_result(env)?;
    let trend = root
        .recommendation_trend
        .and_then(|x| x.trend)
        .unwrap_or_default();

    let rows = trend
        .into_iter()
        .map(|n| super::RecommendationRow {
            period: n.period.unwrap_or_default(),
            strong_buy: n.strong_buy.unwrap_or(0) as u32,
            buy: n.buy.unwrap_or(0) as u32,
            hold: n.hold.unwrap_or(0) as u32,
            sell: n.sell.unwrap_or(0) as u32,
            strong_sell: n.strong_sell.unwrap_or(0) as u32,
        })
        .collect();

    Ok(rows)
}

pub async fn recommendation_summary(
    client: &mut YfClient,
    symbol: &str,
) -> Result<super::RecommendationSummary, YfError> {
    let env = call_quote_summary(client, symbol, "recommendationTrend,recommendationMean").await?;
    let root = get_first_result(env)?;

    let trend = root
        .recommendation_trend
        .and_then(|x| x.trend)
        .unwrap_or_default();

    let latest = trend.first();

    let (latest_period, sb, b, h, s, ss) = match latest {
        Some(t) => (
            t.period.clone(),
            t.strong_buy.unwrap_or(0),
            t.buy.unwrap_or(0),
            t.hold.unwrap_or(0),
            t.sell.unwrap_or(0),
            t.strong_sell.unwrap_or(0),
        ),
        None => (None, 0, 0, 0, 0, 0),
    };

    let (mean, mean_key) = root
        .recommendation_mean
        .map(|m| {
            (
                m.recommendation_mean.and_then(|r| r.raw),
                m.recommendation_key,
            )
        })
        .unwrap_or((None, None));

    Ok(super::RecommendationSummary {
        latest_period,
        strong_buy: sb as u32,
        buy: b as u32,
        hold: h as u32,
        sell: s as u32,
        strong_sell: ss as u32,
        mean,
        mean_key,
    })
}

pub async fn upgrades_downgrades(
    client: &mut YfClient,
    symbol: &str,
) -> Result<Vec<super::UpgradeDowngradeRow>, YfError> {
    let env = call_quote_summary(client, symbol, "upgradeDowngradeHistory").await?;
    let root = get_first_result(env)?;
    let hist = root
        .upgrade_downgrade_history
        .and_then(|x| x.history)
        .unwrap_or_default();

    let mut rows: Vec<super::UpgradeDowngradeRow> = hist
        .into_iter()
        .map(|h| super::UpgradeDowngradeRow {
            ts: h.epoch_grade_date.unwrap_or(0),
            firm: h.firm,
            from_grade: h.from_grade,
            to_grade: h.to_grade,
            action: h.action.or(h.grade_change),
        })
        .collect();

    rows.sort_by_key(|r| r.ts);
    Ok(rows)
}

/* ---------- NEW: analyst price targets ---------- */

pub async fn analyst_price_target(
    client: &mut YfClient,
    symbol: &str,
) -> Result<PriceTarget, YfError> {
    let env = call_quote_summary(client, symbol, "financialData").await?;
    let root = get_first_result(env)?;
    let fd = root
        .financial_data
        .ok_or_else(|| YfError::Data("financialData missing".into()))?;

    let f = |x: Option<RawNum>| x.and_then(|n| n.raw);
    let n = |x: Option<RawNum>| x.and_then(|n| n.raw).map(|v| v.round() as u32);

    Ok(PriceTarget {
        mean: f(fd.target_mean_price),
        high: f(fd.target_high_price),
        low: f(fd.target_low_price),
        number_of_analysts: n(fd.number_of_analyst_opinions),
    })
}

/* ---------- Shared call + helpers ---------- */

async fn call_quote_summary(
    client: &mut YfClient,
    symbol: &str,
    modules: &str,
) -> Result<V10Envelope, YfError> {
    for attempt in 0..=1 {
        client.ensure_credentials().await?;

        let crumb = client
            .crumb()
            .ok_or_else(|| YfError::Data("Crumb is not set".into()))?
            .to_string();

        let mut url = client.base_quote_api().join(symbol)?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("modules", modules);
            qp.append_pair("crumb", &crumb);
        }

        let resp = client.http().get(url.clone()).send().await?;
        let text = net::get_text(resp, "analysis_api", symbol, "json").await?;

        #[cfg(any(debug_assertions, feature = "debug-dumps"))]
        {
            let _ = debug_dump_api(symbol, &text);
        }

        let env: V10Envelope = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => return Err(YfError::Data(format!("quoteSummary json parse: {e}"))),
        };

        if let Some(error) = env.quote_summary.as_ref().and_then(|qs| qs.error.as_ref()) {
            let desc = error.description.to_ascii_lowercase();
            if desc.contains("invalid crumb") && attempt == 0 {
                if std::env::var("YF_DEBUG").ok().as_deref() == Some("1") {
                    eprintln!("YF_DEBUG: Invalid crumb in analysis; refreshing and retrying.");
                }
                client.clear_crumb();
                continue;
            }
            return Err(YfError::Data(format!("yahoo error: {}", error.description)));
        }

        return Ok(env);
    }

    Err(YfError::Data("analysis API call failed after retry".into()))
}

fn get_first_result(env: V10Envelope) -> Result<V10Result, YfError> {
    env.quote_summary
        .and_then(|qs| qs.result)
        .and_then(|mut v| v.pop())
        .ok_or_else(|| YfError::Data("empty quoteSummary result".into()))
}

/* ---------------- Serde mapping (only what we need) ---------------- */

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
    #[serde(rename = "recommendationTrend")]
    recommendation_trend: Option<RecommendationTrendNode>,

    #[serde(rename = "recommendationMean")]
    recommendation_mean: Option<RecommendationMeanNode>,

    #[serde(rename = "upgradeDowngradeHistory")]
    upgrade_downgrade_history: Option<UpgradeDowngradeHistoryNode>,

    #[serde(rename = "financialData")]
    financial_data: Option<FinancialDataNode>,
}

/* --- recommendation trend --- */

#[derive(Deserialize)]
struct RecommendationTrendNode {
    trend: Option<Vec<RecommendationNode>>,
}

#[derive(Deserialize)]
struct RecommendationNode {
    period: Option<String>,

    #[serde(rename = "strongBuy")]
    strong_buy: Option<i64>,
    buy: Option<i64>,
    hold: Option<i64>,
    sell: Option<i64>,

    #[serde(rename = "strongSell")]
    strong_sell: Option<i64>,
}

/* --- recommendation mean / key --- */

#[derive(Deserialize)]
struct RecommendationMeanNode {
    #[serde(rename = "recommendationMean")]
    recommendation_mean: Option<RawNum>,

    #[serde(rename = "recommendationKey")]
    recommendation_key: Option<String>,
}

/* --- upgrades / downgrades --- */

#[derive(Deserialize)]
struct UpgradeDowngradeHistoryNode {
    history: Option<Vec<UpgradeNode>>,
}

#[derive(Deserialize)]
struct UpgradeNode {
    #[serde(rename = "epochGradeDate")]
    epoch_grade_date: Option<i64>,

    firm: Option<String>,

    #[serde(rename = "toGrade")]
    to_grade: Option<String>,

    #[serde(rename = "fromGrade")]
    from_grade: Option<String>,

    action: Option<String>,
    #[serde(rename = "gradeChange")]
    grade_change: Option<String>,
}

/* --- NEW: financial data (price targets) --- */

#[derive(Deserialize)]
struct FinancialDataNode {
    #[serde(rename = "targetMeanPrice")]
    target_mean_price: Option<RawNum>,
    #[serde(rename = "targetHighPrice")]
    target_high_price: Option<RawNum>,
    #[serde(rename = "targetLowPrice")]
    target_low_price: Option<RawNum>,
    #[serde(rename = "numberOfAnalystOpinions")]
    number_of_analyst_opinions: Option<RawNum>,
}

/* --- shared small wrappers --- */

#[derive(Deserialize, Clone, Copy)]
struct RawNum {
    raw: Option<f64>,
}
