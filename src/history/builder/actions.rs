use crate::core::{
    ProjectionContext, ProjectionIssue, YfError, conversions::i64_to_date,
    currency_resolver::ResolvedCurrencyUnit, diagnostics::required_wire_value, wire::JsonDecimal,
};
use crate::history::wire::Events;
use paft::Decimal;
use paft::market::action::Action;
use std::num::NonZeroU32;

pub(super) type ExtractedActions = (Vec<Action>, Vec<(i64, SplitRatio)>);

#[derive(Clone, Copy, Debug)]
pub(super) struct SplitRatio {
    numerator: NonZeroU32,
    denominator: NonZeroU32,
}

impl SplitRatio {
    pub(super) const fn new(numerator: NonZeroU32, denominator: NonZeroU32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    pub(super) const fn numerator(self) -> NonZeroU32 {
        self.numerator
    }

    pub(super) const fn denominator(self) -> NonZeroU32 {
        self.denominator
    }
}

#[allow(clippy::too_many_lines)]
pub fn extract_actions(
    events: Option<&Events>,
    default_currency: Option<&ResolvedCurrencyUnit>,
    ctx: &mut ProjectionContext,
) -> Result<ExtractedActions, YfError> {
    let mut out: Vec<Action> = Vec::new();
    let mut split_events: Vec<(i64, SplitRatio)> = Vec::new();

    let Some(ev) = events else {
        return Ok((out, split_events));
    };

    if let Some(divs) = ev.dividends.as_ref() {
        for (k, d) in divs {
            let Some(ts) = event_timestamp(k, d.date) else {
                ctx.dropped_item(
                    "dividend",
                    Some(k.as_str()),
                    ProjectionIssue::MissingRequiredField { field: "date" },
                )?;
                continue;
            };
            let date = match i64_to_date(ts) {
                Ok(date) => date,
                Err(err) => {
                    ctx.dropped_item(
                        "dividend",
                        Some(k.as_str()),
                        ProjectionIssue::InvalidField {
                            field: "date",
                            details: err.to_string(),
                        },
                    )?;
                    continue;
                }
            };
            let Some(amount) = required_action_amount(ctx, "dividend", k, &d.amount)? else {
                continue;
            };
            let currency = match event_currency(d.currency.as_deref(), default_currency) {
                Ok(Some(currency)) => currency,
                Ok(None) => {
                    ctx.dropped_item(
                        "dividend",
                        Some(k.as_str()),
                        ProjectionIssue::CurrencyUnresolved,
                    )?;
                    continue;
                }
                Err(reason) => {
                    ctx.dropped_item("dividend", Some(k.as_str()), reason)?;
                    continue;
                }
            };
            let Some(amount) = currency.price_from_decimal(amount) else {
                ctx.dropped_item(
                    "dividend",
                    Some(k.as_str()),
                    ProjectionIssue::ConversionFailed {
                        target: "dividend amount",
                    },
                )?;
                continue;
            };
            out.push(Action::Dividend { date, amount });
        }
    }

    if let Some(gains) = ev.capital_gains.as_ref() {
        for (k, g) in gains {
            let Some(ts) = event_timestamp(k, g.date) else {
                ctx.dropped_item(
                    "capital_gain",
                    Some(k.as_str()),
                    ProjectionIssue::MissingRequiredField { field: "date" },
                )?;
                continue;
            };
            let date = match i64_to_date(ts) {
                Ok(date) => date,
                Err(err) => {
                    ctx.dropped_item(
                        "capital_gain",
                        Some(k.as_str()),
                        ProjectionIssue::InvalidField {
                            field: "date",
                            details: err.to_string(),
                        },
                    )?;
                    continue;
                }
            };
            let Some(amount) = required_action_amount(ctx, "capital_gain", k, &g.amount)? else {
                continue;
            };
            let currency = match event_currency(g.currency.as_deref(), default_currency) {
                Ok(Some(currency)) => currency,
                Ok(None) => {
                    ctx.dropped_item(
                        "capital_gain",
                        Some(k.as_str()),
                        ProjectionIssue::CurrencyUnresolved,
                    )?;
                    continue;
                }
                Err(reason) => {
                    ctx.dropped_item("capital_gain", Some(k.as_str()), reason)?;
                    continue;
                }
            };
            let Some(gain) = currency.price_from_decimal(amount) else {
                ctx.dropped_item(
                    "capital_gain",
                    Some(k.as_str()),
                    ProjectionIssue::ConversionFailed {
                        target: "capital gain amount",
                    },
                )?;
                continue;
            };
            out.push(Action::CapitalGain { date, gain });
        }
    }

    if let Some(splits) = ev.splits.as_ref() {
        for (k, s) in splits {
            let Some(ts) = event_timestamp(k, s.date) else {
                ctx.dropped_item(
                    "split",
                    Some(k.as_str()),
                    ProjectionIssue::MissingRequiredField { field: "date" },
                )?;
                continue;
            };
            let date = match i64_to_date(ts) {
                Ok(date) => date,
                Err(err) => {
                    ctx.dropped_item(
                        "split",
                        Some(k.as_str()),
                        ProjectionIssue::InvalidField {
                            field: "date",
                            details: err.to_string(),
                        },
                    )?;
                    continue;
                }
            };
            let Some(split_ratio) = normalize_split_event(s) else {
                ctx.dropped_item(
                    "split",
                    Some(k.as_str()),
                    ProjectionIssue::InvalidField {
                        field: "splitRatio",
                        details: "missing, zero, negative, non-finite, or too large".into(),
                    },
                )?;
                continue;
            };

            out.push(Action::Split {
                date,
                numerator: split_ratio.numerator(),
                denominator: split_ratio.denominator(),
            });

            split_events.push((ts, split_ratio));
        }
    }

    out.sort_by_key(action_sort_key);
    split_events.sort_by_key(|(ts, _)| *ts);

    Ok((out, split_events))
}

const fn action_sort_key(action: &Action) -> (bool, chrono::NaiveDate) {
    match action {
        Action::Dividend { date, .. }
        | Action::Split { date, .. }
        | Action::CapitalGain { date, .. } => (false, *date),
        _ => (true, chrono::NaiveDate::MAX),
    }
}

fn event_currency(
    code: Option<&str>,
    default_currency: Option<&ResolvedCurrencyUnit>,
) -> Result<Option<ResolvedCurrencyUnit>, ProjectionIssue> {
    let Some(code) = code.map(str::trim).filter(|code| !code.is_empty()) else {
        return Ok(default_currency.cloned());
    };

    ResolvedCurrencyUnit::from_code(code)
        .map(Some)
        .ok_or_else(|| ProjectionIssue::InvalidCurrency {
            code: code.to_string(),
        })
}

fn required_action_amount(
    ctx: &mut ProjectionContext,
    item: &'static str,
    key: &str,
    amount: &crate::core::wire::WireValue<JsonDecimal>,
) -> Result<Option<Decimal>, YfError> {
    Ok(required_wire_value(ctx, item, Some(key), "amount", amount)?
        .copied()
        .map(JsonDecimal::into_decimal))
}

fn event_timestamp(key: &str, date: Option<i64>) -> Option<i64> {
    key.parse::<i64>().ok().or(date)
}

fn normalize_split_event(split: &crate::history::wire::SplitEvent) -> Option<SplitRatio> {
    if let (Some(numerator), Some(denominator)) = (split.numerator, split.denominator)
        && let Some(pair) = normalize_split_pair(numerator, denominator)
    {
        return Some(pair);
    }

    split.split_ratio.as_deref().and_then(normalize_split_ratio)
}

fn normalize_split_ratio(ratio: &str) -> Option<SplitRatio> {
    let ratio = ratio.trim();
    for separator in ['/', ':'] {
        if let Some((numerator, denominator)) = ratio.split_once(separator) {
            return normalize_split_pair(
                parse_split_component(numerator)?,
                parse_split_component(denominator)?,
            );
        }
    }

    normalize_split_pair(parse_split_component(ratio)?, Decimal::ONE)
}

fn parse_split_component(value: &str) -> Option<Decimal> {
    crate::core::wire::parse_decimal_lexeme(value).ok()
}

fn normalize_split_pair(numerator: Decimal, denominator: Decimal) -> Option<SplitRatio> {
    let (numerator, numerator_scale) = split_component_parts(numerator)?;
    let (denominator, denominator_scale) = split_component_parts(denominator)?;
    let scale = numerator_scale.max(denominator_scale);

    let numerator = numerator.checked_mul(pow10(scale - numerator_scale)?)?;
    let denominator = denominator.checked_mul(pow10(scale - denominator_scale)?)?;

    let gcd = gcd(numerator, denominator);
    let numerator = numerator / gcd;
    let denominator = denominator / gcd;

    Some(SplitRatio::new(
        NonZeroU32::new(u32::try_from(numerator).ok()?)?,
        NonZeroU32::new(u32::try_from(denominator).ok()?)?,
    ))
}

fn split_component_parts(value: Decimal) -> Option<(u128, u32)> {
    let value = value.normalize();
    let mantissa = value.mantissa();
    if mantissa <= 0 {
        return None;
    }
    Some((u128::try_from(mantissa).ok()?, value.scale()))
}

const fn pow10(exponent: u32) -> Option<u128> {
    10_u128.checked_pow(exponent)
}

const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_ratio_text_uses_exact_decimal_parsing() {
        let ratio = normalize_split_ratio("1.0e2/2.0e1").expect("exact exponent ratio");
        assert_eq!(ratio.numerator().get(), 5);
        assert_eq!(ratio.denominator().get(), 1);

        assert!(normalize_split_ratio("1.23456789012345678901234567895/1").is_none());
    }
}
