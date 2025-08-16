use crate::core::{YfClient, YfError};

use super::fetch::fetch_modules;
use super::model::{PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};
use super::wire::RawNum;

/* ---------- Public entry points (mapping wire → public models) ---------- */

pub async fn recommendation_trend(
    client: &mut YfClient,
    symbol: &str,
) -> Result<Vec<RecommendationRow>, YfError> {
    let root = fetch_modules(client, symbol, "recommendationTrend").await?;

    let trend = root
        .recommendation_trend
        .and_then(|x| x.trend)
        .unwrap_or_default();

    let rows = trend
        .into_iter()
        .map(|n| RecommendationRow {
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
) -> Result<RecommendationSummary, YfError> {
    let root = fetch_modules(client, symbol, "recommendationTrend,recommendationMean").await?;

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

    Ok(RecommendationSummary {
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
) -> Result<Vec<UpgradeDowngradeRow>, YfError> {
    let root = fetch_modules(client, symbol, "upgradeDowngradeHistory").await?;

    let hist = root
        .upgrade_downgrade_history
        .and_then(|x| x.history)
        .unwrap_or_default();

    let mut rows: Vec<UpgradeDowngradeRow> = hist
        .into_iter()
        .map(|h| UpgradeDowngradeRow {
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

/* ---------- Analyst price targets ---------- */

pub async fn analyst_price_target(
    client: &mut YfClient,
    symbol: &str,
) -> Result<PriceTarget, YfError> {
    let root = fetch_modules(client, symbol, "financialData").await?;
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
