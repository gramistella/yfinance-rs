use crate::{
    analysis::model::EarningsTrendRow,
    core::{
        YfClient, YfError,
        client::{CacheMode, RetryConfig},
        wire::RawNumI64,
    },
};

use super::fetch::fetch_modules;
use super::model::{PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};
use super::wire::RawNum;

/* ---------- Public entry points (mapping wire → public models) ---------- */

pub(super) async fn recommendation_trend(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<RecommendationRow>, YfError> {
    let root = fetch_modules(
        client,
        symbol,
        "recommendationTrend",
        cache_mode,
        retry_override,
    )
    .await?;

    let trend = root
        .recommendation_trend
        .and_then(|x| x.trend)
        .unwrap_or_default();

    let rows = trend
        .into_iter()
        .map(|n| RecommendationRow {
            period: n.period.unwrap_or_default(),
            strong_buy: u32::try_from(n.strong_buy.unwrap_or(0)).unwrap_or(0),
            buy: u32::try_from(n.buy.unwrap_or(0)).unwrap_or(0),
            hold: u32::try_from(n.hold.unwrap_or(0)).unwrap_or(0),
            sell: u32::try_from(n.sell.unwrap_or(0)).unwrap_or(0),
            strong_sell: u32::try_from(n.strong_sell.unwrap_or(0)).unwrap_or(0),
        })
        .collect();

    Ok(rows)
}

pub(super) async fn recommendation_summary(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<RecommendationSummary, YfError> {
    let root = fetch_modules(
        client,
        symbol,
        "recommendationTrend,recommendationMean",
        cache_mode,
        retry_override,
    )
    .await?;

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
        strong_buy: u32::try_from(sb).unwrap_or(0),
        buy: u32::try_from(b).unwrap_or(0),
        hold: u32::try_from(h).unwrap_or(0),
        sell: u32::try_from(s).unwrap_or(0),
        strong_sell: u32::try_from(ss).unwrap_or(0),
        mean,
        mean_key,
    })
}

pub(super) async fn upgrades_downgrades(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<UpgradeDowngradeRow>, YfError> {
    let root = fetch_modules(
        client,
        symbol,
        "upgradeDowngradeHistory",
        cache_mode,
        retry_override,
    )
    .await?;

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

pub(super) async fn analyst_price_target(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<PriceTarget, YfError> {
    let root = fetch_modules(client, symbol, "financialData", cache_mode, retry_override).await?;
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

pub(super) async fn earnings_trend(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<EarningsTrendRow>, YfError> {
    let root = fetch_modules(client, symbol, "earningsTrend", cache_mode, retry_override).await?;

    let trend = root
        .earnings_trend
        .and_then(|x| x.trend)
        .unwrap_or_default();

    let rows = trend
        .into_iter()
        .map(|n| {
            let f = |x: Option<RawNum>| x.and_then(|n| n.raw);
            let fi64 = |x: Option<RawNumI64>| x.and_then(|n| n.raw);
            let u = |x: Option<RawNum>| x.and_then(|n| n.raw).map(|v| v.round() as u32);

            let (
                earnings_estimate_avg,
                earnings_estimate_low,
                earnings_estimate_high,
                earnings_estimate_year_ago_eps,
                earnings_estimate_num_analysts,
                earnings_estimate_growth,
            ) = n
                .earnings_estimate
                .map(|e| {
                    (
                        f(e.avg),
                        f(e.low),
                        f(e.high),
                        f(e.year_ago_eps),
                        u(e.num_analysts),
                        f(e.growth),
                    )
                })
                .unwrap_or_default();

            let (
                revenue_estimate_avg,
                revenue_estimate_low,
                revenue_estimate_high,
                revenue_estimate_year_ago_revenue,
                revenue_estimate_num_analysts,
                revenue_estimate_growth,
            ) = n
                .revenue_estimate
                .map(|e| {
                    (
                        fi64(e.avg),
                        fi64(e.low),
                        fi64(e.high),
                        fi64(e.year_ago_revenue),
                        u(e.num_analysts),
                        f(e.growth),
                    )
                })
                .unwrap_or_default();

            let (
                eps_trend_current,
                eps_trend_7_days_ago,
                eps_trend_30_days_ago,
                eps_trend_60_days_ago,
                eps_trend_90_days_ago,
            ) = n
                .eps_trend
                .map(|e| {
                    (
                        f(e.current),
                        f(e.seven_days_ago),
                        f(e.thirty_days_ago),
                        f(e.sixty_days_ago),
                        f(e.ninety_days_ago),
                    )
                })
                .unwrap_or_default();

            let (
                eps_revisions_up_last_7_days,
                eps_revisions_up_last_30_days,
                eps_revisions_down_last_7_days,
                eps_revisions_down_last_30_days,
            ) = n
                .eps_revisions
                .map(|e| {
                    (
                        u(e.up_last_7_days),
                        u(e.up_last_30_days),
                        u(e.down_last_7_days),
                        u(e.down_last_30_days),
                    )
                })
                .unwrap_or_default();

            EarningsTrendRow {
                period: n.period.unwrap_or_default(),
                growth: f(n.growth),
                earnings_estimate_avg,
                earnings_estimate_low,
                earnings_estimate_high,
                earnings_estimate_year_ago_eps,
                earnings_estimate_num_analysts,
                earnings_estimate_growth,
                revenue_estimate_avg,
                revenue_estimate_low,
                revenue_estimate_high,
                revenue_estimate_year_ago_revenue,
                revenue_estimate_num_analysts,
                revenue_estimate_growth,
                eps_trend_current,
                eps_trend_7_days_ago,
                eps_trend_30_days_ago,
                eps_trend_60_days_ago,
                eps_trend_90_days_ago,
                eps_revisions_up_last_7_days,
                eps_revisions_up_last_30_days,
                eps_revisions_down_last_7_days,
                eps_revisions_down_last_30_days,
            }
        })
        .collect();

    Ok(rows)
}
