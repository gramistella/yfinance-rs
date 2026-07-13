use crate::core::{
    ProjectionContext, ProjectionIssue, YfError, currency_resolver::ResolvedCurrencyUnit,
    diagnostics::optional_projected,
};
use paft::Decimal;

pub fn optional_money_u64_with_currency_issue(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    unit: Option<&ResolvedCurrencyUnit>,
    currency_issue: Option<&ProjectionIssue>,
    value: Option<u64>,
    target: &'static str,
) -> Result<Option<paft::money::Money>, YfError> {
    optional_with_unit_with_currency_issue(
        ctx,
        path,
        key,
        CurrencyUnitContext {
            unit,
            issue: currency_issue,
        },
        value,
        target,
        |unit, value| unit.money_from_u64(value).ok(),
    )
}

#[derive(Clone, Copy)]
struct CurrencyUnitContext<'a> {
    unit: Option<&'a ResolvedCurrencyUnit>,
    issue: Option<&'a ProjectionIssue>,
}

pub fn optional_money_i64_with_currency_issue(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    unit: Option<&ResolvedCurrencyUnit>,
    currency_issue: Option<&ProjectionIssue>,
    value: Option<i64>,
    target: &'static str,
) -> Result<Option<paft::money::Money>, YfError> {
    optional_with_unit_with_currency_issue(
        ctx,
        path,
        key,
        CurrencyUnitContext {
            unit,
            issue: currency_issue,
        },
        value,
        target,
        |unit, value| unit.money_from_i64(value).ok(),
    )
}

pub fn optional_money_decimal_with_currency_issue(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    unit: Option<&ResolvedCurrencyUnit>,
    currency_issue: Option<&ProjectionIssue>,
    value: Option<Decimal>,
    target: &'static str,
) -> Result<Option<paft::money::Money>, YfError> {
    optional_with_unit_with_currency_issue(
        ctx,
        path,
        key,
        CurrencyUnitContext {
            unit,
            issue: currency_issue,
        },
        value,
        target,
        |unit, value| unit.money_from_decimal(value).ok(),
    )
}

pub fn optional_price_decimal_with_currency_issue(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    unit: Option<&ResolvedCurrencyUnit>,
    currency_issue: Option<&ProjectionIssue>,
    value: Option<Decimal>,
    target: &'static str,
) -> Result<Option<paft::money::Price>, YfError> {
    optional_with_unit_with_currency_issue(
        ctx,
        path,
        key,
        CurrencyUnitContext {
            unit,
            issue: currency_issue,
        },
        value,
        target,
        ResolvedCurrencyUnit::price_from_decimal,
    )
}

fn optional_with_unit_with_currency_issue<T, U>(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    currency: CurrencyUnitContext<'_>,
    value: Option<T>,
    target: &'static str,
    convert: impl FnOnce(&ResolvedCurrencyUnit, T) -> Option<U>,
) -> Result<Option<U>, YfError> {
    optional_projected(ctx, path, key, value, |value| {
        let Some(unit) = currency.unit else {
            return Err(currency
                .issue
                .cloned()
                .unwrap_or(ProjectionIssue::CurrencyUnresolved));
        };
        convert(unit, value).ok_or(ProjectionIssue::ConversionFailed { target })
    })
}
