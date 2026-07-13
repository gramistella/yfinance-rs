use chrono::{DateTime, Duration, NaiveDate, Utc};
use std::collections::{BTreeMap, btree_map::Entry};

use crate::{
    core::{
        CallOptions, DataQuality, ProjectionContext, ProjectionIssue, YfClient, YfError,
        YfResponse,
        client::{CacheEndpoint, SymbolEndpoint, normalize_symbol},
        conversions::i64_to_date,
        currency_resolver::{
            CurrencyHints, CurrencyPurpose, ReportingCurrencyEvidence, ResolvedCurrencyUnit,
            project_currency_resolution,
        },
        diagnostics::{
            WireProjection, optional_money_decimal_with_currency_issue,
            optional_price_decimal_with_currency_issue, required_period, required_timestamp,
        },
        wire::{BorrowedWireValue, RawDate, RawDecimal, RawNumU64, WireField, WireValue},
    },
    fundamentals::wire::{TimeseriesData, TimeseriesEnvelope},
};
use paft::domain::ReportingPeriod;
use paft::fundamentals::profile::ShareCount;
use paft::money::{Currency, Money};
use url::Url;

use super::fetch::{fetch_modules_body, parse_modules};
use super::{
    BalanceSheetRow, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps, EarningsYear,
    IncomeStatementRow,
};

const SECONDS_PER_DAY: i64 = 24 * 60 * 60;
const STATEMENT_LOOKBACK_DAYS: i64 = 365 * 5;
// Matches Python yfinance's get_shares_full(start=None, end=None) default.
const SHARE_COUNT_LOOKBACK_DAYS: i64 = 548;

fn module_ref<'a, T>(
    ctx: &mut ProjectionContext,
    feature: &'static str,
    field: &'static str,
    value: &'a impl WireField<T>,
) -> Result<Option<&'a T>, YfError> {
    if let Some(details) = value.invalid_details() {
        ctx.provider_feature_unavailable(
            feature,
            ProjectionIssue::InvalidField {
                field,
                details: details.to_string(),
            },
        )?;
        return Ok(None);
    }

    Ok(value.as_ref())
}

fn required_i64_from_wire(
    ctx: &mut ProjectionContext,
    item: &'static str,
    key: Option<&str>,
    field: &'static str,
    value: &WireValue<i64>,
) -> Result<Option<i64>, YfError> {
    match value {
        WireValue::Valid(value) => Ok(Some(*value)),
        WireValue::Missing => {
            ctx.dropped_item(item, key, ProjectionIssue::MissingRequiredField { field })?;
            Ok(None)
        }
        WireValue::Invalid(details) => {
            ctx.dropped_item(
                item,
                key,
                ProjectionIssue::InvalidField {
                    field,
                    details: details.to_string(),
                },
            )?;
            Ok(None)
        }
    }
}

fn required_period_from_wire(
    ctx: &mut ProjectionContext,
    item: &'static str,
    key: Option<&str>,
    field: &'static str,
    value: &WireValue<String>,
) -> Result<Option<ReportingPeriod>, YfError> {
    if let Some(details) = value.invalid_details() {
        ctx.dropped_item(
            item,
            key,
            ProjectionIssue::InvalidField {
                field,
                details: details.to_string(),
            },
        )?;
        return Ok(None);
    }

    required_period(ctx, item, key, field, value.as_ref().map(String::as_str))
}

#[derive(serde::Deserialize)]
struct TimeseriesValueDecimal<'a> {
    #[serde(rename = "currencyCode")]
    currency_code: Option<&'a str>,
    #[serde(rename = "reportedValue")]
    #[serde(default)]
    reported_value: WireValue<RawDecimal>,
}

#[derive(serde::Deserialize)]
struct TimeseriesValueU64 {
    #[serde(rename = "reportedValue")]
    #[serde(default)]
    reported_value: WireValue<RawNumU64>,
}

struct TimeseriesRequest<'a> {
    client: &'a YfClient,
    symbol: &'a str,
    quarterly: bool,
    override_currency: Option<Currency>,
    options: &'a CallOptions,
    keys: &'a [&'static str],
    monetary_keys: &'a [&'static str],
    endpoint_name: &'static str,
}

struct TimeseriesItem<'a, T> {
    key: &'a str,
    values_json: &'a serde_json::value::RawValue,
    rows_map: &'a mut BTreeMap<i64, T>,
    timestamps: &'a [i64],
    prefix: &'a str,
    currency: Option<&'a ResolvedCurrencyUnit>,
    currency_issue: Option<&'a ProjectionIssue>,
    expected_currency_code: Option<&'a str>,
    ignore_value_currency_codes: bool,
    ctx: &'a mut ProjectionContext,
}

async fn fetch_timeseries_data<T, F>(
    request: TimeseriesRequest<'_>,
    mut process_item: F,
) -> Result<YfResponse<Vec<T>>, YfError>
where
    F: for<'a> FnMut(TimeseriesItem<'a, T>) -> Result<(), YfError>,
{
    let TimeseriesRequest {
        client,
        symbol,
        quarterly,
        override_currency,
        options,
        keys,
        monetary_keys,
        endpoint_name,
    } = request;
    let symbol = normalize_symbol(symbol)?;

    let mut ctx = ProjectionContext::new(endpoint_name, options.data_quality());
    let has_currency_override = override_currency.is_some();
    let prefix = if quarterly { "quarterly" } else { "annual" };
    let url = timeseries_url(client, &symbol, prefix, keys)?;
    let body = fetch_timeseries_body(client, &symbol, endpoint_name, prefix, url, options).await?;

    let envelope: TimeseriesEnvelope<'_> = serde_json::from_str(&body).map_err(YfError::json)?;
    let result_vec = timeseries_results(envelope)?;

    if result_vec.is_empty() {
        return Ok(ctx.finish(vec![]));
    }

    let (direct_currency, needs_currency) =
        timeseries_currency_evidence(&result_vec, prefix, monetary_keys);
    let (currency, currency_issue) = if needs_currency {
        let projected_currency = project_currency_resolution(
            &mut ctx,
            &symbol,
            CurrencyPurpose::Reporting,
            direct_currency.as_deref(),
            client
                .resolve_reporting_currency(
                    &symbol,
                    override_currency,
                    ReportingCurrencyEvidence::TimeseriesCurrencyCode(direct_currency.as_deref()),
                    options,
                )
                .await,
        )?;
        let currency_issue = projected_currency.issue().cloned();
        (projected_currency.into_unit(), currency_issue)
    } else {
        (None, None)
    };

    let mut rows_map = BTreeMap::<i64, T>::new();

    for item in result_vec {
        if item_is_empty_metadata(&item) {
            continue;
        }
        let Some(timestamps) = item.timestamp else {
            ctx.dropped_item(
                "timeseries_item",
                None,
                ProjectionIssue::MissingRequiredField { field: "timestamp" },
            )?;
            continue;
        };
        if item.values.is_empty() {
            ctx.dropped_item(
                "timeseries_item",
                None,
                ProjectionIssue::MissingRequiredField { field: "values" },
            )?;
            continue;
        }

        for (key, values_json) in item.values {
            process_item(TimeseriesItem {
                key: &key,
                values_json,
                rows_map: &mut rows_map,
                timestamps: &timestamps,
                prefix,
                currency: currency.as_ref(),
                currency_issue: currency_issue.as_ref(),
                expected_currency_code: direct_currency.as_deref(),
                ignore_value_currency_codes: has_currency_override,
                ctx: &mut ctx,
            })?;
        }
    }

    Ok(ctx.finish(rows_map.into_values().rev().collect()))
}

fn item_is_empty_metadata(item: &TimeseriesData<'_>) -> bool {
    item.meta.is_some() && item.timestamp.is_none() && item.values.is_empty()
}

async fn fetch_timeseries_body(
    client: &YfClient,
    symbol: &str,
    endpoint_name: &'static str,
    prefix: &str,
    url: Url,
    options: &CallOptions,
) -> Result<String, YfError> {
    let endpoint = format!("timeseries_{endpoint_name}_{prefix}");
    let (body, _) = crate::core::net::fetch_text_with_auth_retry(
        client,
        url,
        crate::core::net::AuthFetchConfig {
            auth_mode: crate::core::net::AuthMode::RequiredCrumb,
            cache_endpoint: CacheEndpoint::Fundamentals,
            options,
            cache_body: None,
            endpoint: &endpoint,
            fixture_key: symbol,
            ext: "json",
            retry_on_invalid_crumb_body: true,
            cache_validator: Some(validate_timeseries_body),
        },
        |url| client.http().get(url),
    )
    .await?;

    Ok(body)
}

fn validate_timeseries_body(body: &str) -> Result<(), YfError> {
    let envelope: TimeseriesEnvelope<'_> = serde_json::from_str(body).map_err(YfError::json)?;
    timeseries_results(envelope).map(|_| ())
}

fn timeseries_results(
    envelope: TimeseriesEnvelope<'_>,
) -> Result<Vec<TimeseriesData<'_>>, YfError> {
    reject_timeseries_error(&envelope)?;

    envelope
        .timeseries
        .and_then(|ts| ts.result)
        .ok_or_else(|| YfError::MissingData("missing timeseries result".into()))
}

fn reject_timeseries_error(envelope: &TimeseriesEnvelope<'_>) -> Result<(), YfError> {
    if let Some(error) = envelope
        .timeseries
        .as_ref()
        .and_then(|ts| ts.error.as_ref())
        .or_else(|| {
            envelope
                .finance
                .as_ref()
                .and_then(|finance| finance.error.as_ref())
        })
    {
        let message = timeseries_error_message(error);
        crate::core::logging::trace_error!(
            error = %message,
            "timeseries error"
        );
        return Err(YfError::Api(format!("yahoo error: {message}")));
    }

    Ok(())
}

#[derive(serde::Deserialize)]
struct TimeseriesErrorText<'a> {
    description: Option<&'a str>,
    message: Option<&'a str>,
    code: Option<&'a str>,
}

fn timeseries_error_message(error: &serde_json::value::RawValue) -> String {
    serde_json::from_str::<TimeseriesErrorText<'_>>(error.get())
        .ok()
        .and_then(|error| {
            [error.description, error.message, error.code]
                .into_iter()
                .flatten()
                .map(str::trim)
                .find(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| error.get().to_owned())
}

fn timeseries_url(
    client: &YfClient,
    symbol: &str,
    prefix: &str,
    keys: &[&'static str],
) -> Result<Url, YfError> {
    let type_str = keys
        .iter()
        .map(|key| format!("{prefix}{key}"))
        .collect::<Vec<_>>()
        .join(",");
    let (start_ts, end_ts) = timeseries_window();

    let symbol = normalize_symbol(symbol)?;
    let mut url = client.symbol_url(SymbolEndpoint::Timeseries, &symbol)?;
    url.query_pairs_mut()
        .append_pair("symbol", &symbol)
        .append_pair("type", &type_str)
        .append_pair("period1", &start_ts.to_string())
        .append_pair("period2", &end_ts.to_string());

    Ok(url)
}

fn timeseries_window() -> (i64, i64) {
    window_ending_at_next_utc_midnight(Utc::now(), STATEMENT_LOOKBACK_DAYS)
}

fn shares_window(
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<(i64, i64), YfError> {
    let end_ts = end.map_or_else(
        || next_utc_midnight_timestamp(Utc::now()),
        |dt| dt.timestamp(),
    );
    let start_ts = start.map_or_else(
        || timestamp_days_before(end_ts, SHARE_COUNT_LOOKBACK_DAYS),
        |dt| dt.timestamp(),
    );

    if start_ts >= end_ts {
        return Err(YfError::InvalidDates);
    }

    Ok((start_ts, end_ts))
}

fn window_ending_at_next_utc_midnight(now: DateTime<Utc>, lookback_days: i64) -> (i64, i64) {
    let end_ts = next_utc_midnight_timestamp(now);
    let start_ts = timestamp_days_before(end_ts, lookback_days);

    (start_ts, end_ts)
}

const fn next_utc_midnight_timestamp(now: DateTime<Utc>) -> i64 {
    (now.timestamp().div_euclid(SECONDS_PER_DAY) + 1) * SECONDS_PER_DAY
}

fn timestamp_days_before(end_ts: i64, lookback_days: i64) -> i64 {
    DateTime::from_timestamp(end_ts, 0)
        .and_then(|end| end.checked_sub_signed(Duration::days(lookback_days)))
        .map_or(0, |start| start.timestamp())
}

fn timeseries_currency_evidence(
    result: &[TimeseriesData<'_>],
    prefix: &str,
    monetary_keys: &[&str],
) -> (Option<String>, bool) {
    let mut currency_code: Option<String> = None;
    let mut invalid_currency_code: Option<String> = None;
    let mut needs_currency = false;

    for item in result {
        for (key, values_json) in &item.values {
            let Some(field) = key.strip_prefix(prefix) else {
                continue;
            };
            if !monetary_keys.contains(&field) {
                continue;
            }

            let Ok(values) =
                serde_json::from_str::<Vec<&serde_json::value::RawValue>>(values_json.get())
            else {
                continue;
            };

            for value in values {
                let Ok(value) = serde_json::from_str::<TimeseriesCurrencyProbe<'_>>(value.get())
                else {
                    continue;
                };

                if value
                    .reported_value
                    .and_then(|reported| reported.raw)
                    .is_none()
                {
                    continue;
                }

                needs_currency = true;
                let Some(code) = value
                    .currency_code
                    .map(str::trim)
                    .filter(|code| !code.is_empty())
                else {
                    continue;
                };

                if ResolvedCurrencyUnit::from_code(code).is_some() {
                    currency_code.get_or_insert_with(|| code.to_string());
                } else {
                    invalid_currency_code.get_or_insert_with(|| code.to_string());
                }
            }
        }
    }

    (currency_code.or(invalid_currency_code), needs_currency)
}

#[derive(serde::Deserialize)]
struct TimeseriesCurrencyProbe<'a> {
    #[serde(rename = "currencyCode")]
    currency_code: Option<&'a str>,
    #[serde(rename = "reportedValue")]
    #[serde(default, borrow)]
    reported_value: Option<TimeseriesReportedProbe<'a>>,
}

#[derive(serde::Deserialize)]
struct TimeseriesReportedProbe<'a> {
    #[serde(default, borrow)]
    raw: Option<&'a serde_json::value::RawValue>,
}

fn period_from_timestamp(timestamp: i64) -> Result<ReportingPeriod, YfError> {
    ReportingPeriod::date(i64_to_date(timestamp)?)
        .map_err(|err| YfError::InvalidData(format!("invalid reporting period: {err}")))
}

fn parse_timeseries_values<'de, T>(
    ctx: &mut ProjectionContext,
    key: &str,
    values_json: &'de serde_json::value::RawValue,
) -> Result<Option<Vec<Option<T>>>, YfError>
where
    T: serde::Deserialize<'de>,
{
    let values =
        match serde_json::from_str::<Vec<&'de serde_json::value::RawValue>>(values_json.get()) {
            Ok(values) => values,
            Err(err) => {
                ctx.dropped_item(
                    "timeseries_item",
                    Some(key),
                    ProjectionIssue::InvalidField {
                        field: "values",
                        details: err.to_string(),
                    },
                )?;
                return Ok(None);
            }
        };

    let mut parsed = Vec::with_capacity(values.len());
    for (idx, value) in values.into_iter().enumerate() {
        match serde_json::from_str(value.get()) {
            Ok(value) => parsed.push(Some(value)),
            Err(err) => {
                parsed.push(None);
                let value_key = format!("{key}[{idx}]");
                ctx.dropped_item(
                    "timeseries_value",
                    Some(&value_key),
                    ProjectionIssue::InvalidField {
                        field: "values",
                        details: err.to_string(),
                    },
                )?;
            }
        }
    }

    Ok(Some(parsed))
}

fn parsed_timeseries_value<T>(values: &[Option<T>], idx: usize) -> Option<&T> {
    values.get(idx).and_then(Option::as_ref)
}

fn reported_decimal_at(
    ctx: &mut ProjectionContext,
    values: &[Option<TimeseriesValueDecimal<'_>>],
    idx: usize,
    key: &str,
) -> Result<Option<paft::Decimal>, YfError> {
    let Some(value) = parsed_timeseries_value(values, idx) else {
        return Ok(None);
    };
    value.reported_value.optional_raw_field(
        ctx,
        "timeseries.reportedValue",
        Some(key),
        "reportedValue",
    )
}

fn reported_u64_at(
    ctx: &mut ProjectionContext,
    values: &[Option<TimeseriesValueU64>],
    idx: usize,
    key: &str,
) -> Result<Option<u64>, YfError> {
    let Some(value) = parsed_timeseries_value(values, idx) else {
        return Ok(None);
    };
    value.reported_value.optional_raw_field(
        ctx,
        "timeseries.reportedValue",
        Some(key),
        "reportedValue",
    )
}

fn reported_share_count_at(
    ctx: &mut ProjectionContext,
    values: &[Option<super::wire::TimeseriesValue>],
    idx: usize,
    key: &str,
) -> Result<Option<u64>, YfError> {
    let Some(value) = parsed_timeseries_value(values, idx) else {
        return Ok(None);
    };
    value.reported_value.optional_raw_field(
        ctx,
        "timeseries.reportedValue",
        Some(key),
        "reportedValue",
    )
}

fn timeseries_value_currency_issue(
    value: &TimeseriesValueDecimal<'_>,
    expected_code: Option<&str>,
    ignore_value_currency_codes: bool,
) -> Option<ProjectionIssue> {
    if ignore_value_currency_codes {
        return None;
    }

    let code = value
        .currency_code
        .as_ref()
        .map(|code| code.trim())
        .filter(|code| !code.is_empty())?;

    let Some(unit) = ResolvedCurrencyUnit::from_code(code) else {
        return Some(ProjectionIssue::InvalidCurrency {
            code: code.to_string(),
        });
    };

    if let Some(expected) = expected_code
        .map(str::trim)
        .filter(|expected| !expected.is_empty())
        && let Some(expected_unit) = ResolvedCurrencyUnit::from_code(expected)
        && expected_unit != unit
    {
        return Some(ProjectionIssue::InvalidField {
            field: "currencyCode",
            details: format!("conflicting timeseries currencyCode values: {expected} and {code}"),
        });
    }

    None
}

fn row_for_timestamp<'a, T>(
    ctx: &mut ProjectionContext,
    rows_map: &'a mut BTreeMap<i64, T>,
    timestamp: i64,
    key: &str,
    create: impl FnOnce(ReportingPeriod) -> T,
) -> Result<Option<&'a mut T>, YfError> {
    let period = match period_from_timestamp(timestamp) {
        Ok(period) => period,
        Err(err) => {
            ctx.dropped_item(
                "timeseries_row",
                Some(key),
                ProjectionIssue::InvalidField {
                    field: "timestamp",
                    details: err.to_string(),
                },
            )?;
            return Ok(None);
        }
    };

    match rows_map.entry(timestamp) {
        Entry::Occupied(entry) => Ok(Some(entry.into_mut())),
        Entry::Vacant(entry) => Ok(Some(entry.insert(create(period)))),
    }
}

fn process_statement_money_values<T>(
    item: TimeseriesItem<'_, T>,
    create_row: fn(ReportingPeriod) -> T,
    assign_money: fn(&mut T, &str, Option<Money>),
) -> Result<(), YfError> {
    let TimeseriesItem {
        key,
        values_json,
        rows_map,
        timestamps,
        prefix,
        currency,
        currency_issue,
        expected_currency_code,
        ignore_value_currency_codes,
        ctx,
    } = item;

    let Some(field) = key.strip_prefix(prefix) else {
        return Ok(());
    };

    let Some(values) = parse_timeseries_values::<TimeseriesValueDecimal>(ctx, key, values_json)?
    else {
        return Ok(());
    };

    for (idx, timestamp) in timestamps.iter().enumerate() {
        let row_key = format!("{field}@{timestamp}");
        let Some(value) = reported_decimal_at(ctx, &values, idx, &row_key)? else {
            continue;
        };

        let Some(row) = row_for_timestamp(ctx, rows_map, *timestamp, &row_key, create_row)? else {
            continue;
        };

        let currency_issue_for_value = parsed_timeseries_value(&values, idx).and_then(|parsed| {
            timeseries_value_currency_issue(
                parsed,
                expected_currency_code,
                ignore_value_currency_codes,
            )
        });
        let money = match currency_issue_for_value {
            Some(issue) => {
                ctx.omitted_present_field("timeseries.reportedValue", Some(&row_key), issue)?;
                None
            }
            None => optional_money_decimal_with_currency_issue(
                ctx,
                "timeseries.reportedValue",
                Some(&row_key),
                currency,
                currency_issue,
                Some(value),
                "statement monetary value",
            )?,
        };

        assign_money(row, field, money);
    }

    Ok(())
}

fn process_statement_u64_values<T>(
    item: TimeseriesItem<'_, T>,
    create_row: fn(ReportingPeriod) -> T,
    assign_value: fn(&mut T, &str, Option<u64>),
) -> Result<(), YfError> {
    let TimeseriesItem {
        key,
        values_json,
        rows_map,
        timestamps,
        prefix,
        ctx,
        ..
    } = item;

    let Some(field) = key.strip_prefix(prefix) else {
        return Ok(());
    };

    let Some(values) = parse_timeseries_values::<TimeseriesValueU64>(ctx, key, values_json)? else {
        return Ok(());
    };

    for (idx, timestamp) in timestamps.iter().enumerate() {
        let row_key = format!("{field}@{timestamp}");
        let Some(value) = reported_u64_at(ctx, &values, idx, &row_key)? else {
            continue;
        };

        let Some(row) = row_for_timestamp(ctx, rows_map, *timestamp, &row_key, create_row)? else {
            continue;
        };

        assign_value(row, field, Some(value));
    }

    Ok(())
}

const fn empty_income_statement_row(period: ReportingPeriod) -> IncomeStatementRow {
    IncomeStatementRow {
        period,
        total_revenue: None,
        gross_profit: None,
        operating_income: None,
        net_income: None,
        interest_expense: None,
        income_tax_expense: None,
        depreciation_and_amortization: None,
    }
}

fn assign_income_statement_money(row: &mut IncomeStatementRow, field: &str, money: Option<Money>) {
    match field {
        "TotalRevenue" => row.total_revenue = money,
        "GrossProfit" => row.gross_profit = money,
        "OperatingIncome" => row.operating_income = money,
        "NetIncome" => row.net_income = money,
        "InterestExpense" => row.interest_expense = money,
        "TaxProvision" => row.income_tax_expense = money,
        "DepreciationAndAmortization" => row.depreciation_and_amortization = money,
        _ => {}
    }
}

pub(super) async fn income_statement(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    override_currency: Option<Currency>,
    options: &CallOptions,
) -> Result<YfResponse<Vec<IncomeStatementRow>>, YfError> {
    let keys = [
        "TotalRevenue",
        "GrossProfit",
        "OperatingIncome",
        "NetIncome",
        "InterestExpense",
        "TaxProvision",
        "DepreciationAndAmortization",
    ];
    let endpoint_name = "income_statement";

    let result = fetch_timeseries_data(
        TimeseriesRequest {
            client,
            symbol,
            quarterly,
            override_currency,
            options,
            keys: &keys,
            monetary_keys: &keys,
            endpoint_name,
        },
        |item| {
            process_statement_money_values(
                item,
                empty_income_statement_row,
                assign_income_statement_money,
            )
        },
    )
    .await?;

    Ok(result)
}

const fn empty_balance_sheet_row(period: ReportingPeriod) -> BalanceSheetRow {
    BalanceSheetRow {
        period,
        total_assets: None,
        total_liabilities: None,
        total_equity: None,
        cash: None,
        long_term_debt: None,
        shares_outstanding: None,
        current_assets: None,
        current_liabilities: None,
        accounts_receivable: None,
        inventory: None,
        accounts_payable: None,
        net_property_plant_equipment: None,
        goodwill: None,
        intangible_assets: None,
    }
}

fn assign_balance_sheet_money(row: &mut BalanceSheetRow, field: &str, money: Option<Money>) {
    match field {
        "TotalAssets" => row.total_assets = money,
        "TotalLiabilitiesNetMinorityInterest" => row.total_liabilities = money,
        "StockholdersEquity" => row.total_equity = money,
        "CashAndCashEquivalents" => row.cash = money,
        "LongTermDebt" => row.long_term_debt = money,
        "CurrentAssets" => row.current_assets = money,
        "CurrentLiabilities" => row.current_liabilities = money,
        "AccountsReceivable" => row.accounts_receivable = money,
        "Inventory" => row.inventory = money,
        "AccountsPayable" => row.accounts_payable = money,
        "NetPPE" => row.net_property_plant_equipment = money,
        "Goodwill" => row.goodwill = money,
        "OtherIntangibleAssets" => row.intangible_assets = money,
        _ => {}
    }
}

fn assign_balance_sheet_shares(row: &mut BalanceSheetRow, field: &str, shares: Option<u64>) {
    if field == "OrdinarySharesNumber" {
        row.shares_outstanding = shares;
    }
}

fn process_balance_sheet_item(item: TimeseriesItem<'_, BalanceSheetRow>) -> Result<(), YfError> {
    let Some(field) = item.key.strip_prefix(item.prefix) else {
        return Ok(());
    };

    if field == "OrdinarySharesNumber" {
        process_statement_u64_values(item, empty_balance_sheet_row, assign_balance_sheet_shares)
    } else {
        process_statement_money_values(item, empty_balance_sheet_row, assign_balance_sheet_money)
    }
}

pub(super) async fn balance_sheet(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    override_currency: Option<Currency>,
    options: &CallOptions,
) -> Result<YfResponse<Vec<BalanceSheetRow>>, YfError> {
    let keys = [
        "TotalAssets",
        "TotalLiabilitiesNetMinorityInterest",
        "StockholdersEquity",
        "CashAndCashEquivalents",
        "LongTermDebt",
        "OrdinarySharesNumber",
        "CurrentAssets",
        "CurrentLiabilities",
        "AccountsReceivable",
        "Inventory",
        "AccountsPayable",
        "NetPPE",
        "Goodwill",
        "OtherIntangibleAssets",
    ];
    let monetary_keys = [
        "TotalAssets",
        "TotalLiabilitiesNetMinorityInterest",
        "StockholdersEquity",
        "CashAndCashEquivalents",
        "LongTermDebt",
        "CurrentAssets",
        "CurrentLiabilities",
        "AccountsReceivable",
        "Inventory",
        "AccountsPayable",
        "NetPPE",
        "Goodwill",
        "OtherIntangibleAssets",
    ];
    let endpoint_name = "balance_sheet";

    fetch_timeseries_data(
        TimeseriesRequest {
            client,
            symbol,
            quarterly,
            override_currency,
            options,
            keys: &keys,
            monetary_keys: &monetary_keys,
            endpoint_name,
        },
        process_balance_sheet_item,
    )
    .await
}

const fn empty_cashflow_row(period: ReportingPeriod) -> CashflowRow {
    CashflowRow {
        period,
        operating_cashflow: None,
        capital_expenditures: None,
        free_cash_flow: None,
        net_income: None,
        depreciation_and_amortization: None,
    }
}

fn assign_cashflow_money(row: &mut CashflowRow, field: &str, money: Option<Money>) {
    match field {
        "OperatingCashFlow" => row.operating_cashflow = money,
        "CapitalExpenditure" => row.capital_expenditures = money,
        "FreeCashFlow" => row.free_cash_flow = money,
        "NetIncome" => row.net_income = money,
        "DepreciationAndAmortization" => row.depreciation_and_amortization = money,
        _ => {}
    }
}

pub(super) async fn cashflow(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    override_currency: Option<Currency>,
    options: &CallOptions,
) -> Result<YfResponse<Vec<CashflowRow>>, YfError> {
    let keys = [
        "OperatingCashFlow",
        "CapitalExpenditure",
        "FreeCashFlow",
        "NetIncome",
        "DepreciationAndAmortization",
    ];
    let endpoint_name = "cash_flow";

    let mut result = fetch_timeseries_data(
        TimeseriesRequest {
            client,
            symbol,
            quarterly,
            override_currency,
            options,
            keys: &keys,
            monetary_keys: &keys,
            endpoint_name,
        },
        |item| process_statement_money_values(item, empty_cashflow_row, assign_cashflow_money),
    )
    .await?;

    let mut ctx = ProjectionContext::new(endpoint_name, options.data_quality());
    ctx.extend(result.diagnostics);

    // After filling values, calculate FCF if it's missing.
    for row in &mut result.data {
        if row.free_cash_flow.is_none()
            && let (Some(ocf), Some(capex)) = (
                row.operating_cashflow.clone(),
                row.capital_expenditures.clone(),
            )
        {
            // In timeseries API, capex is negative for cash outflow.
            if let Ok(free_cash_flow) = ocf.try_add(&capex) {
                let key = row.period.to_string();
                ctx.repaired_data(
                    "cash_flow",
                    Some(&key),
                    "inferred missing free cash flow from operating cash flow and capital expenditure",
                )?;
                row.free_cash_flow = Some(free_cash_flow);
            }
        }
    }

    Ok(ctx.finish(result.data))
}

fn earnings_has_monetary_values(earnings: &crate::fundamentals::wire::EarningsNode<'_>) -> bool {
    let decimal_present = |value: &WireValue<crate::core::wire::RawDecimal>| {
        value.as_ref().and_then(|v| v.raw.as_ref()).is_some()
    };
    let eps_present = |value: &WireValue<crate::core::wire::RawDecimal>| {
        value.as_ref().and_then(|v| v.raw.as_ref()).is_some()
    };

    earnings.financials_chart.as_ref().is_some_and(|chart| {
        chart.yearly.as_ref().is_some_and(|rows| {
            rows.iter()
                .any(|row| decimal_present(&row.revenue) || decimal_present(&row.earnings))
        }) || chart.quarterly.as_ref().is_some_and(|rows| {
            rows.iter()
                .any(|row| decimal_present(&row.revenue) || decimal_present(&row.earnings))
        })
    }) || earnings.earnings_chart.as_ref().is_some_and(|chart| {
        chart.quarterly.as_ref().is_some_and(|rows| {
            rows.iter()
                .any(|row| eps_present(&row.actual) || eps_present(&row.estimate))
        })
    })
}

#[allow(clippy::too_many_lines)]
pub(super) async fn earnings(
    client: &YfClient,
    symbol: &str,
    override_currency: Option<Currency>,
    options: &CallOptions,
) -> Result<YfResponse<Earnings>, YfError> {
    let mut ctx = ProjectionContext::new("earnings", options.data_quality());
    let body = fetch_modules_body(client, symbol, "earnings", options).await?;
    let root = parse_modules(&body)?;
    let Some(e) = module_ref(&mut ctx, "earnings", "earnings", &root.earnings)? else {
        ctx.unavailable_feature("earnings")?;
        return Ok(ctx.finish(Earnings::default()));
    };
    let financial_currency = e.financial_currency.optional_cloned_field(
        &mut ctx,
        "earnings.financialCurrency",
        Some(symbol),
        "financialCurrency",
    )?;
    client.store_currency_hints(
        symbol,
        CurrencyHints::from_quote_summary_financial(financial_currency.as_deref()),
    );
    let (currency, currency_issue) = if earnings_has_monetary_values(e) {
        let projected_currency = project_currency_resolution(
            &mut ctx,
            symbol,
            CurrencyPurpose::Reporting,
            financial_currency.as_deref(),
            client
                .resolve_reporting_currency(
                    symbol,
                    override_currency,
                    ReportingCurrencyEvidence::FinancialCurrency(financial_currency.as_deref()),
                    options,
                )
                .await,
        )?;
        let currency_issue = projected_currency.issue().cloned();
        (projected_currency.into_unit(), currency_issue)
    } else {
        (None, None)
    };

    let financials_chart = e.financials_chart.optional_ref_field(
        &mut ctx,
        "earnings.financialsChart",
        None,
        "financialsChart",
    )?;
    let earnings_chart = e.earnings_chart.optional_ref_field(
        &mut ctx,
        "earnings.earningsChart",
        None,
        "earningsChart",
    )?;

    let mut yearly = Vec::new();
    if let Some(financials_chart) = financials_chart
        && let Some(rows) = financials_chart.yearly.optional_ref_field(
            &mut ctx,
            "earnings.financialsChart.yearly",
            None,
            "yearly",
        )?
    {
        for y in rows {
            let Some(date) =
                required_i64_from_wire(&mut ctx, "earnings_year", None, "date", &y.date)?
            else {
                continue;
            };
            let Ok(year) = i32::try_from(date) else {
                let key = date.to_string();
                ctx.dropped_item(
                    "earnings_year",
                    Some(&key),
                    ProjectionIssue::InvalidField {
                        field: "date",
                        details: "year outside i32 range".into(),
                    },
                )?;
                continue;
            };
            let mut row = match EarningsYear::new(year) {
                Ok(row) => row,
                Err(err) => {
                    let key = year.to_string();
                    ctx.dropped_item(
                        "earnings_year",
                        Some(&key),
                        ProjectionIssue::InvalidField {
                            field: "date",
                            details: err.to_string(),
                        },
                    )?;
                    continue;
                }
            };
            let year_key = Some(year.to_string());
            let revenue = y
                .revenue
                .optional_copied_field(
                    &mut ctx,
                    "financialsChart.yearly[].revenue",
                    year_key.as_deref(),
                    "revenue",
                )?
                .and_then(|raw| raw.raw);
            let earnings = y
                .earnings
                .optional_copied_field(
                    &mut ctx,
                    "financialsChart.yearly[].earnings",
                    year_key.as_deref(),
                    "earnings",
                )?
                .and_then(|raw| raw.raw);
            row.revenue = optional_money_decimal_with_currency_issue(
                &mut ctx,
                "financialsChart.yearly[].revenue",
                year_key.as_deref(),
                currency.as_ref(),
                currency_issue.as_ref(),
                revenue,
                "earnings monetary value",
            )?;
            row.earnings = optional_money_decimal_with_currency_issue(
                &mut ctx,
                "financialsChart.yearly[].earnings",
                year_key.as_deref(),
                currency.as_ref(),
                currency_issue.as_ref(),
                earnings,
                "earnings monetary value",
            )?;
            yearly.push(row);
        }
    }

    let mut quarterly = Vec::new();
    if let Some(financials_chart) = financials_chart
        && let Some(rows) = financials_chart.quarterly.optional_ref_field(
            &mut ctx,
            "earnings.financialsChart.quarterly",
            None,
            "quarterly",
        )?
    {
        for q in rows {
            let Some(period) = required_period_from_wire(
                &mut ctx,
                "earnings_quarter",
                q.date.as_ref().map(String::as_str),
                "date",
                &q.date,
            )?
            else {
                continue;
            };
            let period_key = q.date.as_ref().cloned();
            let revenue = q
                .revenue
                .optional_copied_field(
                    &mut ctx,
                    "financialsChart.quarterly[].revenue",
                    period_key.as_deref(),
                    "revenue",
                )?
                .and_then(|raw| raw.raw);
            let earnings = q
                .earnings
                .optional_copied_field(
                    &mut ctx,
                    "financialsChart.quarterly[].earnings",
                    period_key.as_deref(),
                    "earnings",
                )?
                .and_then(|raw| raw.raw);
            quarterly.push(EarningsQuarter {
                period,
                revenue: optional_money_decimal_with_currency_issue(
                    &mut ctx,
                    "financialsChart.quarterly[].revenue",
                    period_key.as_deref(),
                    currency.as_ref(),
                    currency_issue.as_ref(),
                    revenue,
                    "earnings monetary value",
                )?,
                earnings: optional_money_decimal_with_currency_issue(
                    &mut ctx,
                    "financialsChart.quarterly[].earnings",
                    period_key.as_deref(),
                    currency.as_ref(),
                    currency_issue.as_ref(),
                    earnings,
                    "earnings monetary value",
                )?,
            });
        }
    }

    let mut quarterly_eps = Vec::new();
    if let Some(earnings_chart) = earnings_chart
        && let Some(rows) = earnings_chart.quarterly.optional_ref_field(
            &mut ctx,
            "earnings.earningsChart.quarterly",
            None,
            "quarterly",
        )?
    {
        for q in rows {
            let Some(period) = required_period_from_wire(
                &mut ctx,
                "earnings_quarter_eps",
                q.date.as_ref().map(String::as_str),
                "date",
                &q.date,
            )?
            else {
                continue;
            };
            let period_key = q.date.as_ref().cloned();
            let actual = q
                .actual
                .optional_copied_field(
                    &mut ctx,
                    "earningsChart.quarterly[].actual",
                    period_key.as_deref(),
                    "actual",
                )?
                .and_then(|raw| raw.raw);
            let estimate = q
                .estimate
                .optional_copied_field(
                    &mut ctx,
                    "earningsChart.quarterly[].estimate",
                    period_key.as_deref(),
                    "estimate",
                )?
                .and_then(|raw| raw.raw);
            quarterly_eps.push(EarningsQuarterEps {
                period,
                actual: optional_price_decimal_with_currency_issue(
                    &mut ctx,
                    "earningsChart.quarterly[].actual",
                    period_key.as_deref(),
                    currency.as_ref(),
                    currency_issue.as_ref(),
                    actual,
                    "earnings price value",
                )?,
                estimate: optional_price_decimal_with_currency_issue(
                    &mut ctx,
                    "earningsChart.quarterly[].estimate",
                    period_key.as_deref(),
                    currency.as_ref(),
                    currency_issue.as_ref(),
                    estimate,
                    "earnings price value",
                )?,
            });
        }
    }

    Ok(ctx.finish(Earnings {
        yearly,
        quarterly,
        quarterly_eps,
    }))
}

pub(super) async fn calendar(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<YfResponse<super::Calendar>, YfError> {
    let body = fetch_modules_body(client, symbol, "calendarEvents", options).await?;
    let root = parse_modules(&body)?;
    map_calendar(&root, options.data_quality())
}

pub(super) fn calendar_from_quote_summary_raw(
    raw: &serde_json::value::RawValue,
    data_quality: DataQuality,
) -> Result<YfResponse<super::Calendar>, YfError> {
    let root: super::wire::V10Result<'_> =
        serde_json::from_str(raw.get()).map_err(YfError::json)?;
    map_calendar(&root, data_quality)
}

fn map_calendar(
    root: &super::wire::V10Result<'_>,
    data_quality: DataQuality,
) -> Result<YfResponse<super::Calendar>, YfError> {
    let mut ctx = ProjectionContext::new("calendar", data_quality);
    let Some(calendar_events) = module_ref(
        &mut ctx,
        "calendarEvents",
        "calendarEvents",
        &root.calendar_events,
    )?
    else {
        ctx.unavailable_feature("calendarEvents")?;
        return Ok(ctx.finish(super::Calendar {
            earnings_dates: Vec::new(),
            ex_dividend_date: None,
            dividend_payment_date: None,
        }));
    };

    let earnings_dates = calendar_earnings_dates(&mut ctx, &calendar_events.earnings)?;
    let ex_dividend_date = optional_calendar_date(
        &mut ctx,
        "calendarEvents.exDividendDate",
        "exDividendDate",
        &calendar_events.ex_dividend_date,
    )?;
    let dividend_payment_date = optional_calendar_date(
        &mut ctx,
        "calendarEvents.dividendDate",
        "dividendDate",
        &calendar_events.dividend_date,
    )?;

    Ok(ctx.finish(super::Calendar {
        earnings_dates,
        ex_dividend_date,
        dividend_payment_date,
    }))
}

fn calendar_earnings_dates(
    ctx: &mut ProjectionContext,
    earnings: &BorrowedWireValue<'_, super::wire::CalendarEarningsNode<'_>>,
) -> Result<Vec<DateTime<Utc>>, YfError> {
    let mut out = Vec::new();
    let Some(earnings) =
        earnings.optional_ref_field(ctx, "calendarEvents.earnings", None, "earnings")?
    else {
        return Ok(out);
    };
    let Some(dates) = earnings.earnings_date.optional_ref_field(
        ctx,
        "calendarEvents.earnings.earningsDate",
        None,
        "earningsDate",
    )?
    else {
        return Ok(out);
    };

    for (idx, date) in dates.iter().enumerate() {
        let key = Some(idx.to_string());
        if let Some(date) = required_timestamp(
            ctx,
            "calendar_earnings_date",
            key.as_deref(),
            "earningsDate",
            Some(*date),
        )? {
            out.push(date);
        }
    }
    Ok(out)
}

fn optional_calendar_date(
    ctx: &mut ProjectionContext,
    path: &'static str,
    field: &'static str,
    value: &WireValue<RawDate>,
) -> Result<Option<NaiveDate>, YfError> {
    let Some(value) = value.optional_ref_field(ctx, path, None, field)? else {
        return Ok(None);
    };
    let Some(raw) = value.raw else {
        ctx.omitted_present_field(path, None, ProjectionIssue::MissingRequiredField { field })?;
        return Ok(None);
    };
    match i64_to_date(raw) {
        Ok(date) => Ok(Some(date)),
        Err(err) => {
            ctx.omitted_present_field(
                path,
                None,
                ProjectionIssue::InvalidField {
                    field,
                    details: err.to_string(),
                },
            )?;
            Ok(None)
        }
    }
}

pub(super) async fn shares(
    client: &YfClient,
    symbol: &str,
    start: Option<chrono::DateTime<Utc>>,
    end: Option<chrono::DateTime<Utc>>,
    quarterly: bool,
    options: &CallOptions,
) -> Result<YfResponse<Vec<ShareCount>>, YfError> {
    let symbol = normalize_symbol(symbol)?;
    let mut ctx = ProjectionContext::new("shares", options.data_quality());
    let (start_ts, end_ts) = shares_window(start, end)?;

    let type_key = if quarterly {
        "quarterlyOrdinarySharesNumber"
    } else {
        "annualOrdinarySharesNumber"
    };

    let mut url = client.symbol_url(SymbolEndpoint::Timeseries, &symbol)?;
    url.query_pairs_mut()
        .append_pair("symbol", &symbol)
        .append_pair("type", type_key)
        .append_pair("period1", &start_ts.to_string())
        .append_pair("period2", &end_ts.to_string());

    let endpoint = format!("timeseries_{type_key}");
    let (body, _) = crate::core::net::fetch_text_with_auth_retry(
        client,
        url,
        crate::core::net::AuthFetchConfig {
            auth_mode: crate::core::net::AuthMode::RequiredCrumb,
            cache_endpoint: CacheEndpoint::Fundamentals,
            options,
            cache_body: None,
            endpoint: &endpoint,
            fixture_key: &symbol,
            ext: "json",
            retry_on_invalid_crumb_body: true,
            cache_validator: Some(validate_timeseries_body),
        },
        |url| client.http().get(url),
    )
    .await?;

    let envelope: TimeseriesEnvelope<'_> = serde_json::from_str(&body).map_err(YfError::json)?;
    let result_vec = timeseries_results(envelope)?;

    let result_data: Option<TimeseriesData<'_>> = result_vec
        .into_iter()
        .find(|data| data.values.contains_key(type_key));

    let Some(TimeseriesData {
        timestamp: Some(timestamps),
        values: mut values_map,
        ..
    }) = result_data
    else {
        return Ok(ctx.finish(vec![]));
    };

    let Some(values_json) = values_map.remove(type_key) else {
        return Ok(ctx.finish(vec![]));
    };

    let Some(values) =
        parse_timeseries_values::<super::wire::TimeseriesValue>(&mut ctx, type_key, values_json)?
    else {
        return Ok(ctx.finish(Vec::new()));
    };

    let mut counts = Vec::new();
    for (idx, ts) in timestamps.into_iter().enumerate() {
        let row_key = format!("{type_key}@{ts}");
        let Some(shares) = reported_share_count_at(&mut ctx, &values, idx, &row_key)? else {
            continue;
        };
        let date = match i64_to_date(ts) {
            Ok(date) => date,
            Err(err) => {
                let key = format!("{type_key}@{ts}");
                ctx.dropped_item(
                    "share_count",
                    Some(&key),
                    ProjectionIssue::InvalidField {
                        field: "timestamp",
                        details: err.to_string(),
                    },
                )?;
                continue;
            }
        };
        counts.push(ShareCount { date, shares });
    }

    Ok(ctx.finish(counts))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use super::{
        SECONDS_PER_DAY, SHARE_COUNT_LOOKBACK_DAYS, STATEMENT_LOOKBACK_DAYS, YfError,
        next_utc_midnight_timestamp, shares_window, window_ending_at_next_utc_midnight,
    };

    #[test]
    fn statement_window_is_stable_within_utc_day() {
        let morning = Utc.with_ymd_and_hms(2026, 5, 28, 1, 2, 3).unwrap();
        let evening = Utc.with_ymd_and_hms(2026, 5, 28, 23, 59, 59).unwrap();
        let expected_end = Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap();

        let morning_window = window_ending_at_next_utc_midnight(morning, STATEMENT_LOOKBACK_DAYS);
        let evening_window = window_ending_at_next_utc_midnight(evening, STATEMENT_LOOKBACK_DAYS);

        assert_eq!(morning_window, evening_window);
        assert_eq!(morning_window.1, expected_end.timestamp());
        assert_eq!(
            morning_window.1 - morning_window.0,
            STATEMENT_LOOKBACK_DAYS * SECONDS_PER_DAY
        );
    }

    #[test]
    fn next_utc_midnight_advances_at_utc_boundary() {
        let before_midnight = Utc.with_ymd_and_hms(2026, 5, 28, 23, 59, 59).unwrap();
        let at_midnight = Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap();

        assert_eq!(
            next_utc_midnight_timestamp(before_midnight),
            at_midnight.timestamp()
        );
        assert_eq!(
            next_utc_midnight_timestamp(at_midnight),
            Utc.with_ymd_and_hms(2026, 5, 30, 0, 0, 0)
                .unwrap()
                .timestamp()
        );
    }

    #[test]
    fn shares_window_defaults_start_from_effective_end() {
        let end = Utc.with_ymd_and_hms(2026, 5, 28, 12, 34, 56).unwrap();
        let expected_start = end - Duration::days(SHARE_COUNT_LOOKBACK_DAYS);

        let (start_ts, end_ts) = shares_window(None, Some(end)).unwrap();

        assert_eq!(end_ts, end.timestamp());
        assert_eq!(start_ts, expected_start.timestamp());
    }

    #[test]
    fn shares_window_respects_explicit_start() {
        let start = Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 5, 28, 12, 34, 56).unwrap();

        let (start_ts, end_ts) = shares_window(Some(start), Some(end)).unwrap();

        assert_eq!(start_ts, start.timestamp());
        assert_eq!(end_ts, end.timestamp());
    }

    #[test]
    fn shares_window_rejects_inverted_range() {
        let start = Utc.with_ymd_and_hms(2026, 5, 28, 12, 34, 56).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap();

        let err = shares_window(Some(start), Some(end)).unwrap_err();

        assert!(matches!(err, YfError::InvalidDates));
    }
}
