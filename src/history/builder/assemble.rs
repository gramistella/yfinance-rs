use crate::core::conversions::{i64_to_datetime, quantity_from_u64};
use crate::core::currency_resolver::ResolvedCurrencyUnit;
use crate::core::diagnostics::WireProjection;
use crate::core::wire::{JsonDecimal, WireField, WireValue};
use crate::core::{ProjectionContext, ProjectionIssue, YfError};
use crate::history::wire::QuoteBlock;
use paft::Decimal;
use paft::market::responses::history::{Candle, Ohlc};
use paft::money::PriceAmount;

use super::adjust::{
    AdjustmentError, AdjustmentFactor, AdjustmentPlan, SharedAdjustmentFactor,
    provider_adjustment_factor,
};

pub fn assemble_candles(
    ts: &[i64],
    q: &QuoteBlock,
    adjustment_plan: Option<&AdjustmentPlan>,
    split_adjustments: &[Option<SharedAdjustmentFactor>],
    currency: &ResolvedCurrencyUnit,
    ctx: &mut ProjectionContext,
) -> Result<Vec<Candle>, YfError> {
    let mut out = Vec::with_capacity(candle_capacity_upper_bound(ts, q));

    for (i, &t) in ts.iter().enumerate() {
        let ts = match i64_to_datetime(t) {
            Ok(ts) => ts,
            Err(err) => {
                let key = t.to_string();
                ctx.dropped_item(
                    "candle",
                    Some(&key),
                    ProjectionIssue::InvalidField {
                        field: "timestamp",
                        details: err.to_string(),
                    },
                )?;
                continue;
            }
        };
        let volume0 = q.volume.get(i).and_then(|x| *x);

        let (mut open, mut high, mut low, mut close) =
            match raw_ohlc_values(q.open.get(i), q.high.get(i), q.low.get(i), q.close.get(i)) {
                Ok(values) => values,
                Err(reason) => {
                    let key = t.to_string();
                    ctx.dropped_item("candle", Some(&key), reason)?;
                    continue;
                }
            };
        let raw_close = close;

        if let Some(adjustment_plan) = adjustment_plan {
            let Some(factor) = adjustment_plan.factor_for_row(i, split_adjustments) else {
                let key = t.to_string();
                ctx.dropped_item(
                    "candle",
                    Some(&key),
                    ProjectionIssue::InvalidField {
                        field: "adjclose",
                        details: "missing precomputed adjustment factor".into(),
                    },
                )?;
                continue;
            };

            let adjusted = match checked_adjust_ohlc(open, high, low, close, factor) {
                Ok(adjusted) => adjusted,
                Err(error) => {
                    let key = t.to_string();
                    ctx.dropped_item(
                        "candle",
                        Some(&key),
                        ProjectionIssue::ConversionFailed {
                            target: error.diagnostic_target(),
                        },
                    )?;
                    continue;
                }
            };
            (open, high, low, close) = adjusted;
        }

        let Some((open, high, low, close)) = candle_prices(open, high, low, close, currency) else {
            let key = t.to_string();
            ctx.dropped_item(
                "candle",
                Some(&key),
                ProjectionIssue::ConversionFailed {
                    target: "candle prices",
                },
            )?;
            continue;
        };
        let close_unadj = currency.price_amount_from_decimal(raw_close);
        if close_unadj.is_none() {
            let key = t.to_string();
            ctx.omitted_present_field(
                "quote.close_unadj",
                Some(&key),
                ProjectionIssue::ConversionFailed {
                    target: "unadjusted close price",
                },
            )?;
        }
        out.push(Candle {
            ts,
            currency: currency.currency().clone(),
            ohlc: Ohlc::new(open, high, low, close),
            close_unadj,
            volume: volume0.and_then(quantity_from_u64),
            provider: (),
        });
    }

    Ok(out)
}

pub fn adjustment_plan_for_series(
    q: &QuoteBlock,
    adj: &[WireValue<JsonDecimal>],
    len: usize,
    ctx: &mut ProjectionContext,
) -> Result<AdjustmentPlan, YfError> {
    let mut emitted_rows = 0usize;
    let mut provider_adjusted_rows = 0usize;
    let mut row_factors = vec![None; len];

    for (i, row_factor) in row_factors.iter_mut().enumerate().take(len) {
        let Ok((_, _, _, close)) =
            raw_ohlc_values(q.open.get(i), q.high.get(i), q.low.get(i), q.close.get(i))
        else {
            continue;
        };

        emitted_rows += 1;
        let key = i.to_string();
        let adjclose = adj.get(i).map_or(Ok(None), |value| {
            value.optional_copied_map(
                ctx,
                "chart.indicators.adjclose",
                Some(&key),
                JsonDecimal::into_decimal,
            )
        })?;
        if let Some(factor) = provider_adjustment_factor(adjclose, Some(close)) {
            *row_factor = Some(factor);
            provider_adjusted_rows += 1;
        }
    }

    if provider_adjusted_rows == 0 {
        return Ok(AdjustmentPlan::SplitAdjusted);
    }

    if provider_adjusted_rows == emitted_rows {
        return Ok(AdjustmentPlan::ProviderAdjusted { row_factors });
    }

    ctx.repaired_data(
        "candle_adjustment",
        None,
        "ignored sparse chart.indicators.adjclose and used split-only adjustment for all candles",
    )?;

    Ok(AdjustmentPlan::SplitAdjusted)
}

fn candle_capacity_upper_bound(ts: &[i64], q: &QuoteBlock) -> usize {
    [q.open.len(), q.high.len(), q.low.len(), q.close.len()]
        .into_iter()
        .fold(ts.len(), usize::min)
}

fn raw_ohlc_values(
    open: Option<&WireValue<JsonDecimal>>,
    high: Option<&WireValue<JsonDecimal>>,
    low: Option<&WireValue<JsonDecimal>>,
    close: Option<&WireValue<JsonDecimal>>,
) -> Result<(Decimal, Decimal, Decimal, Decimal), ProjectionIssue> {
    let open = decimal_value("open", open)?;
    let high = decimal_value("high", high)?;
    let low = decimal_value("low", low)?;
    let close = decimal_value("close", close)?;

    let mut missing = Vec::with_capacity(4);
    if open.is_none() {
        missing.push("open");
    }
    if high.is_none() {
        missing.push("high");
    }
    if low.is_none() {
        missing.push("low");
    }
    if close.is_none() {
        missing.push("close");
    }
    if !missing.is_empty() {
        return Err(ProjectionIssue::MissingRequiredFields { fields: missing });
    }

    Ok((
        open.expect("checked above"),
        high.expect("checked above"),
        low.expect("checked above"),
        close.expect("checked above"),
    ))
}

fn decimal_value(
    field: &'static str,
    value: Option<&WireValue<JsonDecimal>>,
) -> Result<Option<Decimal>, ProjectionIssue> {
    let Some(value) = value else {
        return Ok(None);
    };
    if let Some(details) = value.invalid_details() {
        return Err(ProjectionIssue::InvalidField {
            field,
            details: details.into_owned(),
        });
    }

    Ok(value.as_ref().copied().map(JsonDecimal::into_decimal))
}

fn candle_prices(
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    currency: &ResolvedCurrencyUnit,
) -> Option<(PriceAmount, PriceAmount, PriceAmount, PriceAmount)> {
    Some((
        currency.price_amount_from_decimal(open)?,
        currency.price_amount_from_decimal(high)?,
        currency.price_amount_from_decimal(low)?,
        currency.price_amount_from_decimal(close)?,
    ))
}

fn checked_adjust_ohlc(
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    factor: &AdjustmentFactor,
) -> Result<(Decimal, Decimal, Decimal, Decimal), AdjustmentError> {
    Ok((
        factor.apply(open)?,
        factor.apply(high)?,
        factor.apply(low)?,
        factor.apply(close)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candle_capacity_upper_bound_uses_shortest_required_array() {
        let quote: QuoteBlock = serde_json::from_str(
            r#"{
                "open":[1,2,3],
                "high":[1,2],
                "low":[1,2,3,4],
                "close":[1],
                "volume":[1,2,3,4,5]
            }"#,
        )
        .unwrap();

        assert_eq!(candle_capacity_upper_bound(&[1, 2, 3, 4, 5], &quote), 1);
    }

    #[test]
    fn provider_adjustment_plan_carries_row_factors() {
        let quote: QuoteBlock = serde_json::from_str(
            r#"{
                "open":[100,101],
                "high":[100,101],
                "low":[100,101],
                "close":[100,101],
                "volume":[1,1]
            }"#,
        )
        .unwrap();
        let adjclose: Vec<WireValue<JsonDecimal>> = serde_json::from_str("[50,101]").unwrap();
        let mut ctx = ProjectionContext::new("history_chart", crate::core::DataQuality::BestEffort);

        let plan = adjustment_plan_for_series(&quote, &adjclose, 2, &mut ctx).unwrap();
        let first = AdjustmentFactor::new(50.into(), 100.into());
        let second = AdjustmentFactor::new(101.into(), 101.into());

        assert_eq!(
            plan.factor_for_row(0, &[]),
            first.as_ref(),
            "provider ratio should stay intact while selecting the provider-adjusted plan"
        );
        assert_eq!(plan.factor_for_row(1, &[]), second.as_ref());
    }
}
