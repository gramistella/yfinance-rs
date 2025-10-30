use chrono::{Duration, Utc};
use std::collections::BTreeMap;

use crate::{
    core::{
        YfClient, YfError,
        client::{CacheMode, RetryConfig},
        conversions::{f64_to_money_with_currency, i64_to_datetime, string_to_period},
    },
    fundamentals::wire::{TimeseriesData, TimeseriesEnvelope},
};
use paft::fundamentals::profile::ShareCount;
use paft::money::Currency;

use super::fetch::fetch_modules;
use super::{
    BalanceSheetRow, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps, EarningsYear,
    IncomeStatementRow,
};

/// Generic helper function to fetch and process timeseries data from the fundamentals API.
///
/// This function handles the common pattern of:
/// 1. Constructing the URL for the /ws/fundamentals-timeseries endpoint
/// 2. Making the request with caching logic
/// 3. Parsing the `TimeseriesEnvelope`
/// 4. Processing the data into a `BTreeMap`
///
/// The `process_item` closure is responsible for processing each timeseries item
/// and updating the rows map accordingly.
#[allow(clippy::too_many_arguments)]
async fn fetch_timeseries_data<T, F>(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
    keys: &[&str],
    endpoint_name: &str,
    _create_default_row: fn(i64) -> T,
    process_item: F,
) -> Result<Vec<T>, YfError>
where
    F: Fn(&str, &serde_json::Value, &mut BTreeMap<i64, T>, &[i64], &str) -> Result<(), YfError>,
{
    let prefix = if quarterly { "quarterly" } else { "annual" };
    let types: Vec<String> = keys.iter().map(|k| format!("{prefix}{k}")).collect();
    let type_str = types.join(",");

    let end_ts = Utc::now().timestamp();
    let start_ts = Utc::now()
        .checked_sub_signed(Duration::days(365 * 5))
        .map_or(0, |dt| dt.timestamp());

    let mut url = client.base_timeseries().join(symbol)?;
    url.query_pairs_mut()
        .append_pair("symbol", symbol)
        .append_pair("type", &type_str)
        .append_pair("period1", &start_ts.to_string())
        .append_pair("period2", &end_ts.to_string());

    client.ensure_credentials().await?;
    if let Some(crumb) = client.crumb().await {
        url.query_pairs_mut().append_pair("crumb", &crumb);
    }

    let body = if cache_mode == CacheMode::Use {
        if let Some(cached) = client.cache_get(&url).await {
            cached
        } else {
            let resp = client
                .send_with_retry(client.http().get(url.clone()), retry_override)
                .await?;
            let endpoint = format!("timeseries_{endpoint_name}_{prefix}");
            let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
            if cache_mode != CacheMode::Bypass {
                client.cache_put(&url, &text, None).await;
            }
            text
        }
    } else {
        let resp = client
            .send_with_retry(client.http().get(url.clone()), retry_override)
            .await?;
        let endpoint = format!("timeseries_{endpoint_name}_{prefix}");
        let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
        if cache_mode != CacheMode::Bypass {
            client.cache_put(&url, &text, None).await;
        }
        text
    };

    let envelope: TimeseriesEnvelope = serde_json::from_str(&body).map_err(YfError::Json)?;

    let result_vec = envelope
        .timeseries
        .and_then(|ts| ts.result)
        .unwrap_or_default();

    if result_vec.is_empty() {
        return Ok(vec![]);
    }

    let mut rows_map = BTreeMap::<i64, T>::new();

    for item in result_vec {
        if let (Some(timestamps), Some((key, values_json))) =
            (item.timestamp, item.values.into_iter().next())
        {
            // Process the item using the provided closure
            process_item(&key, &values_json, &mut rows_map, &timestamps, prefix)?;
        }
    }

    Ok(rows_map.into_values().rev().collect())
}

pub(super) async fn income_statement(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    currency: Currency,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<IncomeStatementRow>, YfError> {
    use serde::Deserialize;

    use crate::core::wire::RawNum;

    #[derive(Deserialize)]
    struct TimeseriesValueF64 {
        #[serde(rename = "reportedValue")]
        reported_value: Option<RawNum<f64>>,
    }

    let keys = [
        "TotalRevenue",
        "GrossProfit",
        "OperatingIncome",
        "NetIncome",
    ];
    let endpoint_name = "income_statement";

    let create_default_row = |period_end: i64| IncomeStatementRow {
        period: string_to_period(&i64_to_datetime(period_end).format("%Y-%m-%d").to_string()),
        total_revenue: None,
        gross_profit: None,
        operating_income: None,
        net_income: None,
    };

    let process_item = |key: &str,
                        values_json: &serde_json::Value,
                        rows_map: &mut BTreeMap<i64, IncomeStatementRow>,
                        timestamps: &[i64],
                        prefix: &str|
     -> Result<(), YfError> {
        if let Ok(values) = serde_json::from_value::<Vec<TimeseriesValueF64>>(values_json.clone()) {
            for (i, ts) in timestamps.iter().enumerate() {
                let row = rows_map
                    .entry(*ts)
                    .or_insert_with(|| create_default_row(*ts));

                let value = values
                    .get(i)
                    .and_then(|v| v.reported_value.and_then(|rv| rv.raw));

                if key == format!("{prefix}TotalRevenue") {
                    row.total_revenue =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}GrossProfit") {
                    row.gross_profit =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}OperatingIncome") {
                    row.operating_income =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}NetIncome") {
                    row.net_income = value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                }
            }
        }
        Ok(())
    };

    let result = fetch_timeseries_data(
        client,
        symbol,
        quarterly,
        cache_mode,
        retry_override,
        &keys,
        endpoint_name,
        create_default_row,
        process_item,
    )
    .await?;

    Ok(result)
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::cognitive_complexity)]
pub(super) async fn balance_sheet(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    currency: Currency,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<BalanceSheetRow>, YfError> {
    use serde::Deserialize;

    use crate::core::wire::{RawNum, RawNumU64};

    #[derive(Deserialize)]
    struct TimeseriesValueF64 {
        #[serde(rename = "reportedValue")]
        reported_value: Option<RawNum<f64>>,
    }
    #[derive(Deserialize)]
    struct TimeseriesValueU64 {
        #[serde(rename = "reportedValue")]
        reported_value: Option<RawNumU64>,
    }

    let keys = [
        "TotalAssets",
        "TotalLiabilitiesNetMinorityInterest",
        "StockholdersEquity",
        "CashAndCashEquivalents",
        "LongTermDebt",
        "OrdinarySharesNumber",
    ];
    let endpoint_name = "balance_sheet";

    let create_default_row = |period_end: i64| BalanceSheetRow {
        period: string_to_period(&i64_to_datetime(period_end).format("%Y-%m-%d").to_string()),
        total_assets: None,
        total_liabilities: None,
        total_equity: None,
        cash: None,
        long_term_debt: None,
        shares_outstanding: None,
    };

    let process_item = |key: &str,
                        values_json: &serde_json::Value,
                        rows_map: &mut BTreeMap<i64, BalanceSheetRow>,
                        timestamps: &[i64],
                        prefix: &str|
     -> Result<(), YfError> {
        if key.ends_with("OrdinarySharesNumber") {
            if let Ok(values) =
                serde_json::from_value::<Vec<TimeseriesValueU64>>(values_json.clone())
            {
                for (i, ts) in timestamps.iter().enumerate() {
                    let row = rows_map
                        .entry(*ts)
                        .or_insert_with(|| create_default_row(*ts));
                    row.shares_outstanding = values
                        .get(i)
                        .and_then(|v| v.reported_value.and_then(|rv| rv.raw));
                }
            }
        } else if let Ok(values) =
            serde_json::from_value::<Vec<TimeseriesValueF64>>(values_json.clone())
        {
            for (i, ts) in timestamps.iter().enumerate() {
                let row = rows_map
                    .entry(*ts)
                    .or_insert_with(|| create_default_row(*ts));

                let value = values
                    .get(i)
                    .and_then(|v| v.reported_value.and_then(|rv| rv.raw));

                if key == format!("{prefix}TotalAssets") {
                    row.total_assets =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}TotalLiabilitiesNetMinorityInterest") {
                    row.total_liabilities =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}StockholdersEquity") {
                    row.total_equity =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}CashAndCashEquivalents") {
                    row.cash = value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}LongTermDebt") {
                    row.long_term_debt =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                }
            }
        }
        Ok(())
    };

    fetch_timeseries_data(
        client,
        symbol,
        quarterly,
        cache_mode,
        retry_override,
        &keys,
        endpoint_name,
        create_default_row,
        process_item,
    )
    .await
}

#[allow(clippy::too_many_lines)]
pub(super) async fn cashflow(
    client: &YfClient,
    symbol: &str,
    quarterly: bool,
    currency: Currency,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<CashflowRow>, YfError> {
    use serde::Deserialize;

    use crate::core::wire::RawNum;

    #[derive(Deserialize)]
    struct TimeseriesValueF64 {
        #[serde(rename = "reportedValue")]
        reported_value: Option<RawNum<f64>>,
    }

    let keys = [
        "OperatingCashFlow",
        "CapitalExpenditure",
        "FreeCashFlow",
        "NetIncome",
    ];
    let endpoint_name = "cash_flow";

    let create_default_row = |period_end: i64| CashflowRow {
        period: string_to_period(&i64_to_datetime(period_end).format("%Y-%m-%d").to_string()),
        operating_cashflow: None,
        capital_expenditures: None,
        free_cash_flow: None,
        net_income: None,
    };

    let process_item = |key: &str,
                        values_json: &serde_json::Value,
                        rows_map: &mut BTreeMap<i64, CashflowRow>,
                        timestamps: &[i64],
                        prefix: &str|
     -> Result<(), YfError> {
        if let Ok(values) = serde_json::from_value::<Vec<TimeseriesValueF64>>(values_json.clone()) {
            for (i, ts) in timestamps.iter().enumerate() {
                let row = rows_map
                    .entry(*ts)
                    .or_insert_with(|| create_default_row(*ts));

                let value = values
                    .get(i)
                    .and_then(|v| v.reported_value.and_then(|rv| rv.raw));

                if key == format!("{prefix}OperatingCashFlow") {
                    row.operating_cashflow =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}CapitalExpenditure") {
                    row.capital_expenditures =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}FreeCashFlow") {
                    row.free_cash_flow =
                        value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                } else if key == format!("{prefix}NetIncome") {
                    row.net_income = value.map(|v| f64_to_money_with_currency(v, currency.clone()));
                }
            }
        }
        Ok(())
    };

    let mut result = fetch_timeseries_data(
        client,
        symbol,
        quarterly,
        cache_mode,
        retry_override,
        &keys,
        endpoint_name,
        create_default_row,
        process_item,
    )
    .await?;

    // After filling values, calculate FCF if it's missing.
    for row in &mut result {
        if row.free_cash_flow.is_none()
            && let (Some(ocf), Some(capex)) = (
                row.operating_cashflow.clone(),
                row.capital_expenditures.clone(),
            )
        {
            // In timeseries API, capex is negative for cash outflow.
            row.free_cash_flow = ocf.try_add(&capex).ok();
        }
    }

    Ok(result)
}

pub(super) async fn earnings(
    client: &YfClient,
    symbol: &str,
    currency: Currency,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Earnings, YfError> {
    let root = fetch_modules(client, symbol, "earnings", cache_mode, retry_override).await?;
    let e = root
        .earnings
        .ok_or_else(|| YfError::MissingData("earnings missing".into()))?;

    let yearly = e
        .financials_chart
        .as_ref()
        .and_then(|fc| fc.yearly.as_ref())
        .map(|v| {
            v.iter()
                .filter_map(|y| {
                    y.date.and_then(|date| {
                        i32::try_from(date).ok().map(|year| EarningsYear {
                            year,
                            revenue: y.revenue.as_ref().and_then(|x| {
                                x.raw
                                    .map(|v| f64_to_money_with_currency(v, currency.clone()))
                            }),
                            earnings: y.earnings.as_ref().and_then(|x| {
                                x.raw
                                    .map(|v| f64_to_money_with_currency(v, currency.clone()))
                            }),
                        })
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let quarterly = e
        .financials_chart
        .as_ref()
        .and_then(|fc| fc.quarterly.as_ref())
        .map(|v| {
            v.iter()
                .map(|q| EarningsQuarter {
                    period: string_to_period(&q.date.clone().unwrap_or_default()),
                    revenue: q.revenue.as_ref().and_then(|x| {
                        x.raw
                            .map(|v| f64_to_money_with_currency(v, currency.clone()))
                    }),
                    earnings: q.earnings.as_ref().and_then(|x| {
                        x.raw
                            .map(|v| f64_to_money_with_currency(v, currency.clone()))
                    }),
                })
                .collect()
        })
        .unwrap_or_default();

    let quarterly_eps = e
        .earnings_chart
        .as_ref()
        .and_then(|ec| ec.quarterly.as_ref())
        .map(|v| {
            v.iter()
                .map(|q| EarningsQuarterEps {
                    period: string_to_period(&q.date.clone().unwrap_or_default()),
                    actual: q.actual.as_ref().and_then(|x| {
                        x.raw
                            .map(|v| f64_to_money_with_currency(v, currency.clone()))
                    }),
                    estimate: q.estimate.as_ref().and_then(|x| {
                        x.raw
                            .map(|v| f64_to_money_with_currency(v, currency.clone()))
                    }),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Earnings {
        yearly,
        quarterly,
        quarterly_eps,
    })
}

pub(super) async fn calendar(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<super::Calendar, YfError> {
    let root = fetch_modules(client, symbol, "calendarEvents", cache_mode, retry_override).await?;
    let calendar_events = root
        .calendar_events
        .ok_or_else(|| YfError::MissingData("calendarEvents missing".into()))?;

    let earnings_dates = calendar_events
        .earnings
        .and_then(|e| e.earnings_date)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|d| d.raw.map(i64_to_datetime))
        .collect();

    Ok(super::Calendar {
        earnings_dates,
        ex_dividend_date: calendar_events
            .ex_dividend_date
            .and_then(|x| x.raw.map(i64_to_datetime)),
        dividend_payment_date: calendar_events
            .dividend_date
            .and_then(|x| x.raw.map(i64_to_datetime)),
    })
}

pub(super) async fn shares(
    client: &YfClient,
    symbol: &str,
    start: Option<chrono::DateTime<Utc>>,
    end: Option<chrono::DateTime<Utc>>,
    quarterly: bool,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<Vec<ShareCount>, YfError> {
    let end_ts = end.unwrap_or_else(Utc::now).timestamp();
    let start_ts = start
        .unwrap_or_else(|| Utc::now() - Duration::days(548))
        .timestamp();

    let type_key = if quarterly {
        "quarterlyBasicAverageShares"
    } else {
        "annualBasicAverageShares"
    };

    let mut url = client.base_timeseries().join(symbol)?;
    url.query_pairs_mut()
        .append_pair("symbol", symbol)
        .append_pair("type", type_key)
        .append_pair("period1", &start_ts.to_string())
        .append_pair("period2", &end_ts.to_string());

    client.ensure_credentials().await?;
    if let Some(crumb) = client.crumb().await {
        url.query_pairs_mut().append_pair("crumb", &crumb);
    }

    let body = if cache_mode == CacheMode::Use {
        if let Some(cached) = client.cache_get(&url).await {
            cached
        } else {
            let resp = client
                .send_with_retry(client.http().get(url.clone()), retry_override)
                .await?;
            let endpoint = format!("timeseries_{type_key}");
            let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
            if cache_mode != CacheMode::Bypass {
                client.cache_put(&url, &text, None).await;
            }
            text
        }
    } else {
        let resp = client
            .send_with_retry(client.http().get(url.clone()), retry_override)
            .await?;
        let endpoint = format!("timeseries_{type_key}");
        let text = crate::core::net::get_text(resp, &endpoint, symbol, "json").await?;
        if cache_mode != CacheMode::Bypass {
            client.cache_put(&url, &text, None).await;
        }
        text
    };

    let envelope: TimeseriesEnvelope = serde_json::from_str(&body).map_err(YfError::Json)?;

    let result_data: Option<TimeseriesData> = envelope
        .timeseries
        .and_then(|ts| ts.result)
        .and_then(|mut v| v.pop());

    let Some(TimeseriesData {
        timestamp: Some(timestamps),
        values: mut values_map,
        ..
    }) = result_data
    else {
        return Ok(vec![]);
    };

    let Some(values_json) = values_map.remove(type_key) else {
        return Ok(vec![]);
    };

    let values: Vec<super::wire::TimeseriesValue> =
        serde_json::from_value(values_json).map_err(YfError::Json)?;

    let counts = timestamps
        .into_iter()
        .zip(values.into_iter())
        .filter_map(|(ts, val)| {
            val.reported_value
                .and_then(|rv| rv.raw)
                .map(|shares| ShareCount {
                    date: i64_to_datetime(ts),
                    shares,
                })
        })
        .collect();

    Ok(counts)
}
