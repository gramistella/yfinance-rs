use crate::{
    analysis::model::EarningsTrendRow,
    core::{
        YfClient, YfError,
        client::{CacheMode, RetryConfig},
        wire::{from_raw, from_raw_u32_round},
        conversions::*,
    },
};

use super::fetch::fetch_modules;
use super::model::{PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};
use paft::fundamentals::analysis::{EarningsEstimate, RevenueEstimate, EpsTrend, EpsRevisions, TrendPoint, RevisionPoint};

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
            period: string_to_period(n.period.unwrap_or_default()),
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
        "recommendationTrend,financialData",
        cache_mode,
        retry_override,
    )
    .await?;

    let trend = root
        .recommendation_trend
        .and_then(|x| x.trend)
        .unwrap_or_default();

    let latest = trend.first();

    let (latest_period, sb, b, h, s, ss) = latest.map_or((None, 0, 0, 0, 0, 0), |t| {
        (
            Some(string_to_period(t.period.clone().unwrap_or_default())),
            t.strong_buy.unwrap_or(0),
            t.buy.unwrap_or(0),
            t.hold.unwrap_or(0),
            t.sell.unwrap_or(0),
            t.strong_sell.unwrap_or(0),
        )
    });

    let (mean, _mean_key) = root.financial_data.map_or((None, None), |fd| {
        (from_raw(fd.recommendation_mean), fd.recommendation_key)
    });

    Ok(RecommendationSummary {
        latest_period,
        strong_buy: u32::try_from(sb).unwrap_or(0),
        buy: u32::try_from(b).unwrap_or(0),
        hold: u32::try_from(h).unwrap_or(0),
        sell: u32::try_from(s).unwrap_or(0),
        strong_sell: u32::try_from(ss).unwrap_or(0),
        mean,
        mean_rating_text: None,
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
            ts: i64_to_datetime(h.epoch_grade_date.unwrap_or(0)),
            firm: h.firm,
            from_grade: h.from_grade.map(|g| string_to_recommendation_grade(g)),
            to_grade: h.to_grade.map(|g| string_to_recommendation_grade(g)),
            action: h.action.or(h.grade_change).map(|a| string_to_recommendation_action(a)),
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
        .ok_or_else(|| YfError::MissingData("financialData missing".into()))?;

    Ok(PriceTarget {
        mean: from_raw(fd.target_mean_price).map(f64_to_money),
        high: from_raw(fd.target_high_price).map(f64_to_money),
        low: from_raw(fd.target_low_price).map(f64_to_money),
        number_of_analysts: from_raw_u32_round(fd.number_of_analyst_opinions),
    })
}

#[allow(clippy::too_many_lines)]
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
                        from_raw(e.avg),
                        from_raw(e.low),
                        from_raw(e.high),
                        from_raw(e.year_ago_eps),
                        from_raw_u32_round(e.num_analysts),
                        from_raw(e.growth),
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
                        from_raw(e.avg),
                        from_raw(e.low),
                        from_raw(e.high),
                        from_raw(e.year_ago_revenue),
                        from_raw_u32_round(e.num_analysts),
                        from_raw(e.growth),
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
                        from_raw(e.current),
                        from_raw(e.seven_days_ago),
                        from_raw(e.thirty_days_ago),
                        from_raw(e.sixty_days_ago),
                        from_raw(e.ninety_days_ago),
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
                        from_raw_u32_round(e.up_last_7_days),
                        from_raw_u32_round(e.up_last_30_days),
                        from_raw_u32_round(e.down_last_7_days),
                        from_raw_u32_round(e.down_last_30_days),
                    )
                })
                .unwrap_or_default();

            EarningsTrendRow {
                period: string_to_period(n.period.unwrap_or_default()),
                growth: from_raw(n.growth),
                earnings_estimate: EarningsEstimate {
                    avg: earnings_estimate_avg.map(|v| f64_to_money(v as f64)),
                    low: earnings_estimate_low.map(|v| f64_to_money(v as f64)),
                    high: earnings_estimate_high.map(|v| f64_to_money(v as f64)),
                    year_ago_eps: earnings_estimate_year_ago_eps.map(|v| f64_to_money(v as f64)),
                    num_analysts: earnings_estimate_num_analysts,
                    growth: earnings_estimate_growth,
                },
                revenue_estimate: RevenueEstimate {
                    avg: revenue_estimate_avg.map(|v| f64_to_money(v as f64)),
                    low: revenue_estimate_low.map(|v| f64_to_money(v as f64)),
                    high: revenue_estimate_high.map(|v| f64_to_money(v as f64)),
                    year_ago_revenue: revenue_estimate_year_ago_revenue.map(|v| f64_to_money(v as f64)),
                    num_analysts: revenue_estimate_num_analysts,
                    growth: revenue_estimate_growth,
                },
                eps_trend: EpsTrend {
                    current: eps_trend_current.map(|v| f64_to_money(v as f64)),
                    historical: vec![
                        TrendPoint::new("7d", f64_to_money(eps_trend_7_days_ago.unwrap_or(0.0))),
                        TrendPoint::new("30d", f64_to_money(eps_trend_30_days_ago.unwrap_or(0.0))),
                        TrendPoint::new("60d", f64_to_money(eps_trend_60_days_ago.unwrap_or(0.0))),
                        TrendPoint::new("90d", f64_to_money(eps_trend_90_days_ago.unwrap_or(0.0))),
                    ],
                },
                eps_revisions: EpsRevisions {
                    historical: vec![
                        RevisionPoint::new("7d", eps_revisions_up_last_7_days.unwrap_or(0), eps_revisions_down_last_7_days.unwrap_or(0)),
                        RevisionPoint::new("30d", eps_revisions_up_last_30_days.unwrap_or(0), eps_revisions_down_last_30_days.unwrap_or(0)),
                    ],
                },
            }
        })
        .collect();

    Ok(rows)
}
