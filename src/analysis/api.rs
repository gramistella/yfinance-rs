use crate::{
    analysis::model::EarningsTrendRow,
    core::{
        YfClient, YfError,
        client::{CacheMode, RetryConfig},
        conversions::{
            f64_to_decimal_safely, f64_to_money_with_currency, i64_to_datetime,
            i64_to_money_with_currency, string_to_period, string_to_recommendation_action,
            string_to_recommendation_grade,
        },
        wire::{from_raw, from_raw_u32_round},
    },
};

use super::fetch::fetch_modules;
use super::model::{PriceTarget, RecommendationRow, RecommendationSummary, UpgradeDowngradeRow};
use chrono::DateTime;
use paft::fundamentals::analysis::{
    EarningsEstimate, EpsRevisions, EpsTrend, RevenueEstimate, RevisionPoint, TrendPoint,
};
use paft::money::Currency;
// Period is available via prelude or directly; we use string_to_period for parsing, so import not needed

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
            period: string_to_period(&n.period.unwrap_or_default()),
            strong_buy: n.strong_buy.and_then(|v| u32::try_from(v).ok()),
            buy: n.buy.and_then(|v| u32::try_from(v).ok()),
            hold: n.hold.and_then(|v| u32::try_from(v).ok()),
            sell: n.sell.and_then(|v| u32::try_from(v).ok()),
            strong_sell: n.strong_sell.and_then(|v| u32::try_from(v).ok()),
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

    let (latest_period, sb, b, h, s, ss) =
        latest.map_or((None, None, None, None, None, None), |t| {
            (
                Some(string_to_period(&t.period.clone().unwrap_or_default())),
                t.strong_buy.and_then(|v| u32::try_from(v).ok()),
                t.buy.and_then(|v| u32::try_from(v).ok()),
                t.hold.and_then(|v| u32::try_from(v).ok()),
                t.sell.and_then(|v| u32::try_from(v).ok()),
                t.strong_sell.and_then(|v| u32::try_from(v).ok()),
            )
        });

    let (mean, _mean_key) = root.financial_data.map_or((None, None), |fd| {
        (from_raw(fd.recommendation_mean), fd.recommendation_key)
    });

    Ok(RecommendationSummary {
        latest_period,
        strong_buy: sb,
        buy: b,
        hold: h,
        sell: s,
        strong_sell: ss,
        mean: mean.map(|v| f64_to_decimal_safely(v)),
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
            ts: h.epoch_grade_date.map_or_else(
                || DateTime::from_timestamp(0, 0).unwrap_or_default(),
                i64_to_datetime,
            ),
            firm: h.firm,
            from_grade: h.from_grade.as_deref().map(string_to_recommendation_grade),
            to_grade: h.to_grade.as_deref().map(string_to_recommendation_grade),
            action: h
                .action
                .or(h.grade_change)
                .as_deref()
                .map(string_to_recommendation_action),
        })
        .collect();

    rows.sort_by_key(|r| r.ts);
    Ok(rows)
}

pub(super) async fn analyst_price_target(
    client: &YfClient,
    symbol: &str,
    currency: Currency,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<PriceTarget, YfError> {
    let root = fetch_modules(client, symbol, "financialData", cache_mode, retry_override).await?;
    let fd = root
        .financial_data
        .ok_or_else(|| YfError::MissingData("financialData missing".into()))?;

    Ok(PriceTarget {
        mean: from_raw(fd.target_mean_price)
            .map(|v| f64_to_money_with_currency(v, currency.clone())),
        high: from_raw(fd.target_high_price)
            .map(|v| f64_to_money_with_currency(v, currency.clone())),
        low: from_raw(fd.target_low_price).map(|v| f64_to_money_with_currency(v, currency.clone())),
        number_of_analysts: from_raw_u32_round(fd.number_of_analyst_opinions),
    })
}

#[allow(clippy::too_many_lines)]
pub(super) async fn earnings_trend(
    client: &YfClient,
    symbol: &str,
    currency: Currency,
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
                period: string_to_period(&n.period.unwrap_or_default()),
                growth: from_raw(n.growth).map(|v| f64_to_decimal_safely(v)),
                earnings_estimate: EarningsEstimate {
                    avg: earnings_estimate_avg
                        .map(|v| f64_to_money_with_currency(v, currency.clone())),
                    low: earnings_estimate_low
                        .map(|v| f64_to_money_with_currency(v, currency.clone())),
                    high: earnings_estimate_high
                        .map(|v| f64_to_money_with_currency(v, currency.clone())),
                    year_ago_eps: earnings_estimate_year_ago_eps
                        .map(|v| f64_to_money_with_currency(v, currency.clone())),
                    num_analysts: earnings_estimate_num_analysts,
                    growth: earnings_estimate_growth.map(|v| f64_to_decimal_safely(v)),
                },
                revenue_estimate: RevenueEstimate {
                    avg: revenue_estimate_avg
                        .map(|v| i64_to_money_with_currency(v, currency.clone())),
                    low: revenue_estimate_low
                        .map(|v| i64_to_money_with_currency(v, currency.clone())),
                    high: revenue_estimate_high
                        .map(|v| i64_to_money_with_currency(v, currency.clone())),
                    year_ago_revenue: revenue_estimate_year_ago_revenue
                        .map(|v| i64_to_money_with_currency(v, currency.clone())),
                    num_analysts: revenue_estimate_num_analysts,
                    growth: revenue_estimate_growth.map(|v| f64_to_decimal_safely(v)),
                },
                eps_trend: EpsTrend {
                    current: eps_trend_current
                        .map(|v| f64_to_money_with_currency(v, currency.clone())),
                    historical: {
                        let mut hist = Vec::new();
                        if let Some(v) = eps_trend_7_days_ago
                            && let Ok(tp) = TrendPoint::try_new_str(
                                "7d",
                                f64_to_money_with_currency(v, currency.clone()),
                            )
                        {
                            hist.push(tp);
                        }
                        if let Some(v) = eps_trend_30_days_ago
                            && let Ok(tp) = TrendPoint::try_new_str(
                                "30d",
                                f64_to_money_with_currency(v, currency.clone()),
                            )
                        {
                            hist.push(tp);
                        }
                        if let Some(v) = eps_trend_60_days_ago
                            && let Ok(tp) = TrendPoint::try_new_str(
                                "60d",
                                f64_to_money_with_currency(v, currency.clone()),
                            )
                        {
                            hist.push(tp);
                        }
                        if let Some(v) = eps_trend_90_days_ago
                            && let Ok(tp) = TrendPoint::try_new_str(
                                "90d",
                                f64_to_money_with_currency(v, currency.clone()),
                            )
                        {
                            hist.push(tp);
                        }
                        hist
                    },
                },
                eps_revisions: EpsRevisions {
                    historical: {
                        let mut hist = Vec::new();
                        if let (Some(up), Some(down)) =
                            (eps_revisions_up_last_7_days, eps_revisions_down_last_7_days)
                            && let Ok(rp) = RevisionPoint::try_new_str("7d", up, down)
                        {
                            hist.push(rp);
                        }
                        if let (Some(up), Some(down)) = (
                            eps_revisions_up_last_30_days,
                            eps_revisions_down_last_30_days,
                        ) && let Ok(rp) = RevisionPoint::try_new_str("30d", up, down)
                        {
                            hist.push(rp);
                        }
                        hist
                    },
                },
            }
        })
        .collect();

    Ok(rows)
}
