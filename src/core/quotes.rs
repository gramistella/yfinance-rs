// src/core/quotes.rs
use std::{borrow::Cow, fmt::Write as _};

use futures::{StreamExt, stream};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::{
    YfClient, YfError,
    core::{
        CallOptions, DataQuality, ProjectionContext, ProjectionIssue,
        client::{CacheEndpoint, normalize_symbols},
        conversions::{i64_to_date, i64_to_datetime, quantity_from_u64},
        currency_resolver::{CurrencyHints, ResolvedCurrencyUnit},
        diagnostics::{WireProjection, optional_projected},
        models::{FastInfo, MovingAverages},
        net, quotesummary,
        wire::{JsonDecimal, JsonU64, RawDate, RawDecimal, RawNumU64, WireField, WireValue},
        yahoo_vocab::{first_parsed_yahoo_exchange, parse_yahoo_exchange, parse_yahoo_quote_type},
    },
};
use paft::Decimal;
use paft::aggregates::Snapshot;
use paft::domain::{Exchange, Instrument, MarketState};
use paft::fundamentals::statements::Calendar;
use paft::fundamentals::statistics::KeyStatistics;
use paft::market::orderbook::BookLevel;
use paft::market::quote::Quote;
use paft::money::{Currency, PriceAmount};

const KEY_STATISTICS_MODULES: &str = "summaryDetail,defaultKeyStatistics";
const MAX_V7_QUOTE_SYMBOLS_PER_REQUEST: usize = 100;
const MAX_V7_QUOTE_URL_BYTES: usize = 1_800;
// Live probes plateaued at 12 concurrent quote chunks; 16 added burst without improving latency.
const MAX_V7_QUOTE_CONCURRENT_REQUESTS: usize = 12;

// Centralized wire model for the v7 quote API
#[derive(Deserialize)]
pub struct V7Envelope {
    #[serde(rename = "quoteResponse")]
    pub(crate) quote_response: Option<V7QuoteResponse>,
}

#[derive(Deserialize)]
pub struct V7QuoteResponse {
    pub(crate) result: Option<Vec<Value>>,
    pub(crate) error: Option<V7Error>,
}

#[derive(Deserialize)]
pub struct V7Error {
    pub(crate) description: String,
}

#[derive(Deserialize, Clone)]
pub struct V7QuoteNode {
    #[serde(default)]
    pub(crate) symbol: WireValue<String>,
    #[serde(rename = "quoteType")]
    #[serde(default)]
    pub(crate) quote_type: WireValue<String>,
    #[serde(rename = "shortName")]
    #[serde(default)]
    pub(crate) short_name: WireValue<String>,
    #[serde(rename = "longName")]
    #[serde(default)]
    pub(crate) long_name: WireValue<String>,
    #[serde(rename = "regularMarketPrice")]
    #[serde(default)]
    pub(crate) regular_market_price: WireValue<JsonDecimal>,
    #[serde(rename = "regularMarketOpen")]
    #[serde(default)]
    pub(crate) regular_market_open: WireValue<JsonDecimal>,
    #[serde(rename = "regularMarketDayHigh")]
    #[serde(default)]
    pub(crate) regular_market_day_high: WireValue<JsonDecimal>,
    #[serde(rename = "regularMarketDayLow")]
    #[serde(default)]
    pub(crate) regular_market_day_low: WireValue<JsonDecimal>,
    #[serde(rename = "regularMarketPreviousClose")]
    #[serde(default)]
    pub(crate) regular_market_previous_close: WireValue<JsonDecimal>,
    #[serde(rename = "regularMarketVolume")]
    #[serde(default)]
    pub(crate) regular_market_volume: WireValue<JsonU64>,
    #[serde(default)]
    pub(crate) bid: WireValue<JsonDecimal>,
    #[serde(rename = "bidSize")]
    #[serde(default)]
    pub(crate) bid_size: WireValue<JsonU64>,
    #[serde(default)]
    pub(crate) ask: WireValue<JsonDecimal>,
    #[serde(rename = "askSize")]
    #[serde(default)]
    pub(crate) ask_size: WireValue<JsonU64>,
    #[serde(rename = "regularMarketTime")]
    #[serde(default)]
    pub(crate) regular_market_time: WireValue<i64>,
    #[serde(rename = "averageDailyVolume3Month")]
    #[serde(default)]
    pub(crate) average_daily_volume_3_month: WireValue<JsonU64>,
    #[serde(rename = "fiftyDayAverage")]
    #[serde(default)]
    pub(crate) fifty_day_average: WireValue<JsonDecimal>,
    #[serde(rename = "twoHundredDayAverage")]
    #[serde(default)]
    pub(crate) two_hundred_day_average: WireValue<JsonDecimal>,
    #[serde(rename = "fiftyTwoWeekHigh")]
    #[serde(default)]
    pub(crate) fifty_two_week_high: WireValue<JsonDecimal>,
    #[serde(rename = "fiftyTwoWeekLow")]
    #[serde(default)]
    pub(crate) fifty_two_week_low: WireValue<JsonDecimal>,
    #[serde(rename = "marketCap")]
    #[serde(default)]
    pub(crate) market_cap: WireValue<JsonDecimal>,
    #[serde(rename = "sharesOutstanding")]
    #[serde(default)]
    pub(crate) shares_outstanding: WireValue<JsonU64>,
    #[serde(rename = "epsTrailingTwelveMonths")]
    #[serde(default)]
    pub(crate) eps_trailing_twelve_months: WireValue<JsonDecimal>,
    #[serde(rename = "trailingPE")]
    #[serde(default)]
    pub(crate) trailing_pe: WireValue<JsonDecimal>,
    #[serde(rename = "trailingAnnualDividendYield")]
    #[serde(default)]
    pub(crate) trailing_annual_dividend_yield: WireValue<JsonDecimal>,
    #[serde(rename = "dividendRate")]
    #[serde(default)]
    pub(crate) dividend_rate: WireValue<JsonDecimal>,
    #[serde(rename = "dividendYield")]
    #[serde(default)]
    pub(crate) dividend_yield: WireValue<JsonDecimal>,
    #[serde(default)]
    pub(crate) beta: WireValue<JsonDecimal>,
    #[serde(rename = "dividendDate")]
    #[serde(default)]
    pub(crate) dividend_date: WireValue<i64>,
    #[serde(default)]
    pub(crate) currency: WireValue<String>,
    #[serde(rename = "financialCurrency")]
    #[serde(default)]
    pub(crate) financial_currency: WireValue<String>,
    #[serde(rename = "fullExchangeName")]
    #[serde(default)]
    pub(crate) full_exchange_name: WireValue<String>,
    #[serde(default)]
    pub(crate) exchange: WireValue<String>,
    #[serde(default)]
    pub(crate) market: WireValue<String>,
    #[serde(rename = "marketCapFigureExchange")]
    #[serde(default)]
    pub(crate) market_cap_figure_exchange: WireValue<String>,
    #[serde(rename = "marketState")]
    #[serde(default)]
    pub(crate) market_state: WireValue<String>,
}

fn required_wire_str_projection<'a>(
    value: &'a WireValue<String>,
    field: &'static str,
) -> Result<&'a str, ProjectionIssue> {
    match value {
        WireValue::Valid(value) => {
            nonempty(value).ok_or(ProjectionIssue::MissingRequiredField { field })
        }
        WireValue::Missing => Err(ProjectionIssue::MissingRequiredField { field }),
        WireValue::Invalid(details) => Err(ProjectionIssue::InvalidField {
            field,
            details: details.to_string(),
        }),
    }
}

struct V7KeyStatisticsFields {
    market_cap: Option<Decimal>,
    shares_outstanding: Option<u64>,
    eps_trailing_twelve_months: Option<Decimal>,
    trailing_pe: Option<Decimal>,
    dividend_rate: Option<Decimal>,
    trailing_annual_dividend_yield: Option<Decimal>,
    dividend_yield: Option<Decimal>,
    fifty_two_week_high: Option<Decimal>,
    fifty_two_week_low: Option<Decimal>,
    average_daily_volume_3m: Option<u64>,
    beta: Option<Decimal>,
}

impl V7QuoteNode {
    fn symbol_key(&self) -> Option<String> {
        self.symbol.as_ref().cloned()
    }

    fn currency_units(&self) -> QuoteCurrencyUnits {
        QuoteCurrencyUnits::from_quote_node(self)
    }

    fn exchange_candidates(&self) -> [(&'static str, Option<&str>); 4] {
        [
            ("fullExchangeName", self.full_exchange_name.as_str()),
            ("exchange", self.exchange.as_str()),
            ("market", self.market.as_str()),
            (
                "marketCapFigureExchange",
                self.market_cap_figure_exchange.as_str(),
            ),
        ]
    }

    fn exchange(&self) -> Option<Exchange> {
        first_parsed_yahoo_exchange(
            self.exchange_candidates()
                .into_iter()
                .map(|(_, value)| value),
        )
    }

    fn exchange_with_context(
        &self,
        ctx: &mut ProjectionContext,
        key: Option<&str>,
    ) -> Result<Option<Exchange>, YfError> {
        let candidates = [
            (
                "fullExchangeName",
                self.full_exchange_name
                    .optional_cloned(ctx, "fullExchangeName", key)?,
            ),
            (
                "exchange",
                self.exchange.optional_cloned(ctx, "exchange", key)?,
            ),
            ("market", self.market.optional_cloned(ctx, "market", key)?),
            (
                "marketCapFigureExchange",
                self.market_cap_figure_exchange.optional_cloned(
                    ctx,
                    "marketCapFigureExchange",
                    key,
                )?,
            ),
        ];
        let mut failures = Vec::new();
        for (path, value) in candidates {
            let Some(value) = value.as_deref().and_then(nonempty) else {
                continue;
            };
            match parse_yahoo_exchange(value) {
                Ok(parsed) => return Ok(Some(parsed)),
                Err(err) => failures.push(ExchangeCandidateFailure {
                    path,
                    value: value.to_string(),
                    reason: err.to_string(),
                }),
            }
        }

        if let Some(first) = failures.first() {
            ctx.omitted_present_field(
                first.path,
                key,
                ProjectionIssue::InvalidField {
                    field: first.path,
                    details: exchange_candidate_failure_details(&failures),
                },
            )?;
        }
        Ok(None)
    }

    fn instrument_projection(
        &self,
        exchange: Option<paft::domain::Exchange>,
    ) -> Result<Instrument, ProjectionIssue> {
        let sym = required_wire_str_projection(&self.symbol, "symbol")?;
        let quote_type = required_wire_str_projection(&self.quote_type, "quoteType")?;
        let kind =
            parse_yahoo_quote_type(quote_type).map_err(|err| ProjectionIssue::InvalidField {
                field: "quoteType",
                details: err.to_string(),
            })?;

        let instrument = match exchange {
            Some(ex) => Instrument::from_symbol_and_exchange(sym, ex, kind),
            None => Instrument::from_symbol(sym, kind),
        };

        instrument.map_err(|err| ProjectionIssue::InvalidField {
            field: "symbol",
            details: err.to_string(),
        })
    }

    fn instrument(&self, exchange: Option<paft::domain::Exchange>) -> Result<Instrument, YfError> {
        self.instrument_projection(exchange)
            .map_err(|issue| self.instrument_error(issue))
    }

    fn instrument_error(&self, issue: ProjectionIssue) -> YfError {
        match issue {
            ProjectionIssue::MissingRequiredField { field } => {
                YfError::MissingData(format!("v7 quote node missing {field}"))
            }
            ProjectionIssue::InvalidField {
                field: "symbol",
                details,
            } => YfError::InvalidData(format!(
                "invalid v7 quote symbol {:?}: {details}",
                self.symbol.as_str().unwrap_or_default()
            )),
            ProjectionIssue::InvalidField { field, details } => {
                YfError::InvalidData(format!("invalid v7 quote {field}: {details}"))
            }
            other => YfError::InvalidData(format!("invalid v7 quote instrument: {other}")),
        }
    }

    fn positive_book_level(
        &self,
        ctx: &mut ProjectionContext,
        path: &'static str,
        key: Option<&str>,
        price: Option<Decimal>,
        size: Option<u64>,
    ) -> Result<Option<BookLevel>, YfError> {
        let Some(price) = price.filter(|price| *price > Decimal::ZERO) else {
            return Ok(None);
        };
        let price = self.currency_units().quote_price_amount(
            ctx,
            path,
            key,
            Some(price),
            "quote book level price",
        )?;
        Ok(price.map(|price| BookLevel::new(price, size.and_then(quantity_from_u64))))
    }

    fn market_state_with_context(
        &self,
        ctx: &mut ProjectionContext,
        key: Option<&str>,
    ) -> Result<Option<MarketState>, YfError> {
        let Some(value) = self
            .market_state
            .optional_cloned(ctx, "marketState", key)?
            .and_then(|value| nonempty(&value).map(str::to_owned))
        else {
            return Ok(None);
        };
        match value.as_str().parse() {
            Ok(state) => Ok(Some(state)),
            Err(err) => {
                ctx.omitted_present_field(
                    "marketState",
                    key,
                    ProjectionIssue::InvalidField {
                        field: "marketState",
                        details: err.to_string(),
                    },
                )?;
                Ok(None)
            }
        }
    }

    fn as_of_with_context(
        &self,
        ctx: &mut ProjectionContext,
        key: Option<&str>,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, YfError> {
        let Some(timestamp) =
            self.regular_market_time
                .optional_copied(ctx, "regularMarketTime", key)?
        else {
            return Ok(None);
        };
        match i64_to_datetime(timestamp) {
            Ok(timestamp) => Ok(Some(timestamp)),
            Err(err) => {
                ctx.omitted_present_field(
                    "regularMarketTime",
                    key,
                    ProjectionIssue::InvalidField {
                        field: "regularMarketTime",
                        details: err.to_string(),
                    },
                )?;
                Ok(None)
            }
        }
    }

    pub(crate) fn to_snapshot_with_context(
        &self,
        ctx: &mut ProjectionContext,
    ) -> Result<Snapshot, YfError> {
        let key = self.symbol_key();
        let exchange = self.exchange_with_context(ctx, key.as_deref())?;
        let currencies = self.currency_units();
        let currency = currencies
            .quote_currency()
            .map_err(|issue| YfError::InvalidData(format!("invalid snapshot currency: {issue}")))?;
        let name = self
            .long_name
            .optional_cloned(ctx, "longName", key.as_deref())?
            .or(self
                .short_name
                .optional_cloned(ctx, "shortName", key.as_deref())?);
        let regular_market_price = self.regular_market_price.optional_copied_map(
            ctx,
            "regularMarketPrice",
            key.as_deref(),
            JsonDecimal::into_decimal,
        )?;
        let regular_market_previous_close =
            self.regular_market_previous_close.optional_copied_map(
                ctx,
                "regularMarketPreviousClose",
                key.as_deref(),
                JsonDecimal::into_decimal,
            )?;
        let regular_market_open = self.regular_market_open.optional_copied_map(
            ctx,
            "regularMarketOpen",
            key.as_deref(),
            JsonDecimal::into_decimal,
        )?;
        let regular_market_day_high = self.regular_market_day_high.optional_copied_map(
            ctx,
            "regularMarketDayHigh",
            key.as_deref(),
            JsonDecimal::into_decimal,
        )?;
        let regular_market_day_low = self.regular_market_day_low.optional_copied_map(
            ctx,
            "regularMarketDayLow",
            key.as_deref(),
            JsonDecimal::into_decimal,
        )?;
        let regular_market_volume = self.regular_market_volume.optional_copied_map(
            ctx,
            "regularMarketVolume",
            key.as_deref(),
            JsonU64::into_u64,
        )?;

        Ok(Snapshot {
            instrument: self.instrument(exchange)?,
            name,
            market_state: self.market_state_with_context(ctx, key.as_deref())?,
            as_of: self.as_of_with_context(ctx, key.as_deref())?,
            currency,
            last: currencies.quote_price_amount(
                ctx,
                "regularMarketPrice",
                key.as_deref(),
                regular_market_price,
                "snapshot last price",
            )?,
            previous_close: currencies.quote_price_amount(
                ctx,
                "regularMarketPreviousClose",
                key.as_deref(),
                regular_market_previous_close,
                "snapshot previous close",
            )?,
            open: currencies.quote_price_amount(
                ctx,
                "regularMarketOpen",
                key.as_deref(),
                regular_market_open,
                "snapshot open",
            )?,
            day_high: currencies.quote_price_amount(
                ctx,
                "regularMarketDayHigh",
                key.as_deref(),
                regular_market_day_high,
                "snapshot day high",
            )?,
            day_low: currencies.quote_price_amount(
                ctx,
                "regularMarketDayLow",
                key.as_deref(),
                regular_market_day_low,
                "snapshot day low",
            )?,
            volume: regular_market_volume.and_then(quantity_from_u64),
            provider: (),
        })
    }

    pub(crate) fn to_fast_info_with_context(
        &self,
        ctx: &mut ProjectionContext,
    ) -> Result<FastInfo, YfError> {
        Ok(FastInfo {
            snapshot: self.to_snapshot_with_context(ctx)?,
            moving_averages: self.to_moving_averages_with_context(ctx)?,
        })
    }

    pub(crate) fn to_moving_averages_with_context(
        &self,
        ctx: &mut ProjectionContext,
    ) -> Result<MovingAverages, YfError> {
        let key = self.symbol_key();
        let currencies = self.currency_units();
        let fifty_day = self.fifty_day_average.optional_copied_map(
            ctx,
            "fiftyDayAverage",
            key.as_deref(),
            JsonDecimal::into_decimal,
        )?;
        let two_hundred_day = self.two_hundred_day_average.optional_copied_map(
            ctx,
            "twoHundredDayAverage",
            key.as_deref(),
            JsonDecimal::into_decimal,
        )?;

        Ok(MovingAverages {
            fifty_day: currencies.quote_price(
                ctx,
                "fiftyDayAverage",
                key.as_deref(),
                fifty_day,
                "50-day moving average",
            )?,
            two_hundred_day: currencies.quote_price(
                ctx,
                "twoHundredDayAverage",
                key.as_deref(),
                two_hundred_day,
                "200-day moving average",
            )?,
        })
    }

    pub(crate) fn to_key_statistics_with_context(
        &self,
        ctx: &mut ProjectionContext,
    ) -> Result<KeyStatistics, YfError> {
        let currencies = self.currency_units();
        let key = self.symbol_key();
        let fields = self.key_statistics_fields(ctx, key.as_deref())?;
        let dividend_yield_forward = optional_projected(
            ctx,
            "dividendYield",
            key.as_deref(),
            fields.dividend_yield,
            |value| {
                let fraction = value.checked_div(Decimal::from(100)).ok_or(
                    ProjectionIssue::ConversionFailed {
                        target: "forward dividend yield fraction",
                    },
                )?;
                if fraction.checked_mul(Decimal::from(100)) == Some(value) {
                    Ok(fraction)
                } else {
                    Err(ProjectionIssue::ConversionFailed {
                        target: "exact forward dividend yield fraction",
                    })
                }
            },
        )?;

        Ok(KeyStatistics {
            as_of: self.as_of_with_context(ctx, key.as_deref())?,
            market_cap: currencies.quote_money(
                ctx,
                "marketCap",
                key.as_deref(),
                fields.market_cap,
                "market cap",
            )?,
            shares_outstanding: fields.shares_outstanding,
            eps_trailing_twelve_months: currencies.financial_price(
                ctx,
                "epsTrailingTwelveMonths",
                key.as_deref(),
                fields.eps_trailing_twelve_months,
                "trailing EPS",
            )?,
            pe_trailing_twelve_months: fields.trailing_pe,
            dividend_per_share_forward: currencies.quote_major_price(
                ctx,
                "dividendRate",
                key.as_deref(),
                fields.dividend_rate,
                "forward dividend per share",
            )?,
            dividend_yield_trailing: fields.trailing_annual_dividend_yield,
            // Yahoo v7 returns trailingAnnualDividendYield as a decimal fraction,
            // but dividendYield as percent points. Keep this asymmetry fixture-locked.
            dividend_yield_forward,
            ex_dividend_date: None,
            fifty_two_week_high: currencies.quote_price(
                ctx,
                "fiftyTwoWeekHigh",
                key.as_deref(),
                fields.fifty_two_week_high,
                "52-week high",
            )?,
            fifty_two_week_low: currencies.quote_price(
                ctx,
                "fiftyTwoWeekLow",
                key.as_deref(),
                fields.fifty_two_week_low,
                "52-week low",
            )?,
            average_daily_volume_3m: fields.average_daily_volume_3m,
            beta: fields.beta,
        })
    }

    fn key_statistics_fields(
        &self,
        ctx: &mut ProjectionContext,
        key: Option<&str>,
    ) -> Result<V7KeyStatisticsFields, YfError> {
        Ok(V7KeyStatisticsFields {
            market_cap: self.market_cap.optional_copied_map(
                ctx,
                "marketCap",
                key,
                JsonDecimal::into_decimal,
            )?,
            shares_outstanding: self.shares_outstanding.optional_copied_map(
                ctx,
                "sharesOutstanding",
                key,
                JsonU64::into_u64,
            )?,
            eps_trailing_twelve_months: self.eps_trailing_twelve_months.optional_copied_map(
                ctx,
                "epsTrailingTwelveMonths",
                key,
                JsonDecimal::into_decimal,
            )?,
            trailing_pe: self.trailing_pe.optional_copied_map(
                ctx,
                "trailingPE",
                key,
                JsonDecimal::into_decimal,
            )?,
            dividend_rate: self.dividend_rate.optional_copied_map(
                ctx,
                "dividendRate",
                key,
                JsonDecimal::into_decimal,
            )?,
            trailing_annual_dividend_yield: self
                .trailing_annual_dividend_yield
                .optional_copied_map(
                    ctx,
                    "trailingAnnualDividendYield",
                    key,
                    JsonDecimal::into_decimal,
                )?,
            dividend_yield: self.dividend_yield.optional_copied_map(
                ctx,
                "dividendYield",
                key,
                JsonDecimal::into_decimal,
            )?,
            fifty_two_week_high: self.fifty_two_week_high.optional_copied_map(
                ctx,
                "fiftyTwoWeekHigh",
                key,
                JsonDecimal::into_decimal,
            )?,
            fifty_two_week_low: self.fifty_two_week_low.optional_copied_map(
                ctx,
                "fiftyTwoWeekLow",
                key,
                JsonDecimal::into_decimal,
            )?,
            average_daily_volume_3m: self.average_daily_volume_3_month.optional_copied_map(
                ctx,
                "averageDailyVolume3Month",
                key,
                JsonU64::into_u64,
            )?,
            beta: self
                .beta
                .optional_copied_map(ctx, "beta", key, JsonDecimal::into_decimal)?,
        })
    }

    pub(crate) fn calendar_fallback_with_context(
        &self,
        ctx: &mut ProjectionContext,
    ) -> Result<Option<Calendar>, YfError> {
        let key = self.symbol_key();
        let Some(timestamp) =
            self.dividend_date
                .optional_copied(ctx, "dividendDate", key.as_deref())?
        else {
            return Ok(None);
        };
        match i64_to_date(timestamp) {
            Ok(date) => Ok(Some(Calendar {
                earnings_dates: Vec::new(),
                ex_dividend_date: None,
                dividend_payment_date: Some(date),
            })),
            Err(err) => {
                ctx.omitted_present_field(
                    "dividendDate",
                    key.as_deref(),
                    ProjectionIssue::InvalidField {
                        field: "dividendDate",
                        details: err.to_string(),
                    },
                )?;
                Ok(None)
            }
        }
    }
}

struct ExchangeCandidateFailure {
    path: &'static str,
    value: String,
    reason: String,
}

fn exchange_candidate_failure_details(failures: &[ExchangeCandidateFailure]) -> String {
    let mut details = String::from("no exchange candidate parsed");
    for failure in failures {
        let _ = write!(
            details,
            "; {}={:?}: {}",
            failure.path, failure.value, failure.reason
        );
    }
    details
}

#[derive(Clone)]
struct QuoteCurrencyUnits {
    quote: Option<ResolvedCurrencyUnit>,
    quote_issue: Option<ProjectionIssue>,
    quote_major: Option<ResolvedCurrencyUnit>,
    financial: Option<ResolvedCurrencyUnit>,
    financial_issue: Option<ProjectionIssue>,
}

impl QuoteCurrencyUnits {
    fn from_quote_node(node: &V7QuoteNode) -> Self {
        let (quote, quote_issue) = parse_currency_unit(
            node.currency.as_str(),
            node.currency.invalid_details(),
            "currency",
            false,
        );
        let quote_major = quote.as_ref().map(ResolvedCurrencyUnit::major_unit);
        let (financial, financial_issue) = node.financial_currency.invalid_details().map_or_else(
            || {
                node.financial_currency
                    .as_str()
                    .and_then(nonempty)
                    .map_or_else(
                        || (quote_major.clone(), quote_issue.clone()),
                        |code| parse_currency_unit(Some(code), None, "financialCurrency", true),
                    )
            },
            |details| parse_currency_unit(None, Some(details), "financialCurrency", true),
        );

        Self {
            quote,
            quote_issue,
            quote_major,
            financial,
            financial_issue,
        }
    }

    fn from_quote_summary_currency(currency: Option<&str>) -> Self {
        let (quote, quote_issue) = parse_currency_unit(currency, None, "currency", false);
        let quote_major = quote.as_ref().map(ResolvedCurrencyUnit::major_unit);
        let financial = quote_major.clone();

        Self {
            quote,
            quote_issue: quote_issue.clone(),
            quote_major,
            financial,
            financial_issue: quote_issue,
        }
    }

    fn quote_price(
        &self,
        ctx: &mut ProjectionContext,
        path: &'static str,
        key: Option<&str>,
        value: Option<Decimal>,
        target: &'static str,
    ) -> Result<Option<paft::money::Price>, YfError> {
        optional_with_unit(
            ctx,
            path,
            key,
            self.quote_unit(),
            value,
            target,
            ResolvedCurrencyUnit::price_from_decimal,
        )
    }

    fn quote_price_amount(
        &self,
        ctx: &mut ProjectionContext,
        path: &'static str,
        key: Option<&str>,
        value: Option<Decimal>,
        target: &'static str,
    ) -> Result<Option<PriceAmount>, YfError> {
        optional_with_unit(
            ctx,
            path,
            key,
            self.quote_unit(),
            value,
            target,
            ResolvedCurrencyUnit::price_amount_from_decimal,
        )
    }

    fn quote_money(
        &self,
        ctx: &mut ProjectionContext,
        path: &'static str,
        key: Option<&str>,
        value: Option<Decimal>,
        target: &'static str,
    ) -> Result<Option<paft::money::Money>, YfError> {
        optional_with_unit(
            ctx,
            path,
            key,
            self.quote_major_unit(),
            value,
            target,
            |unit, value| unit.money_from_decimal(value).ok(),
        )
    }

    fn quote_major_price(
        &self,
        ctx: &mut ProjectionContext,
        path: &'static str,
        key: Option<&str>,
        value: Option<Decimal>,
        target: &'static str,
    ) -> Result<Option<paft::money::Price>, YfError> {
        optional_with_unit(
            ctx,
            path,
            key,
            self.quote_major_unit(),
            value,
            target,
            ResolvedCurrencyUnit::price_from_decimal,
        )
    }

    fn financial_price(
        &self,
        ctx: &mut ProjectionContext,
        path: &'static str,
        key: Option<&str>,
        value: Option<Decimal>,
        target: &'static str,
    ) -> Result<Option<paft::money::Price>, YfError> {
        optional_with_unit(
            ctx,
            path,
            key,
            self.financial_unit(),
            value,
            target,
            ResolvedCurrencyUnit::price_from_decimal,
        )
    }

    fn quote_unit(&self) -> Result<&ResolvedCurrencyUnit, ProjectionIssue> {
        self.quote.as_ref().ok_or_else(|| self.quote_issue())
    }

    fn quote_currency(&self) -> Result<Currency, ProjectionIssue> {
        self.quote_unit().map(|unit| unit.currency().clone())
    }

    fn quote_major_unit(&self) -> Result<&ResolvedCurrencyUnit, ProjectionIssue> {
        self.quote_major.as_ref().ok_or_else(|| self.quote_issue())
    }

    fn financial_unit(&self) -> Result<&ResolvedCurrencyUnit, ProjectionIssue> {
        self.financial.as_ref().ok_or_else(|| {
            self.financial_issue
                .clone()
                .unwrap_or(ProjectionIssue::CurrencyUnresolved)
        })
    }

    fn quote_issue(&self) -> ProjectionIssue {
        self.quote_issue
            .clone()
            .unwrap_or(ProjectionIssue::CurrencyUnresolved)
    }
}

fn nonempty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn parse_currency_unit(
    code: Option<&str>,
    invalid_details: Option<Cow<'_, str>>,
    field: &'static str,
    major: bool,
) -> (Option<ResolvedCurrencyUnit>, Option<ProjectionIssue>) {
    if let Some(details) = invalid_details {
        return (
            None,
            Some(ProjectionIssue::InvalidField {
                field,
                details: details.to_string(),
            }),
        );
    }

    let Some(code) = code.and_then(nonempty) else {
        return (None, None);
    };
    let unit = if major {
        ResolvedCurrencyUnit::major_from_code(code)
    } else {
        ResolvedCurrencyUnit::from_code(code)
    };
    unit.map_or_else(
        || {
            (
                None,
                Some(ProjectionIssue::InvalidCurrency {
                    code: code.to_string(),
                }),
            )
        },
        |unit| (Some(unit), None),
    )
}

fn optional_with_unit<T, U>(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    unit: Result<&ResolvedCurrencyUnit, ProjectionIssue>,
    value: Option<T>,
    target: &'static str,
    convert: impl FnOnce(&ResolvedCurrencyUnit, T) -> Option<U>,
) -> Result<Option<U>, YfError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let unit = match unit {
        Ok(unit) => unit,
        Err(issue) => {
            ctx.omitted_present_field(path, key, issue)?;
            return Ok(None);
        }
    };
    let Some(converted) = convert(unit, value) else {
        ctx.omitted_present_field(path, key, ProjectionIssue::ConversionFailed { target })?;
        return Ok(None);
    };
    Ok(Some(converted))
}

#[derive(Deserialize)]
pub struct QuoteSummaryKeyStatistics {
    #[serde(rename = "summaryDetail")]
    summary_detail: Option<SummaryDetailNode>,
    #[serde(rename = "defaultKeyStatistics")]
    default_key_statistics: Option<DefaultKeyStatisticsNode>,
}

#[derive(Default, Deserialize)]
struct SummaryDetailNode {
    #[serde(default)]
    currency: WireValue<String>,
    #[serde(default)]
    beta: WireValue<RawDecimal>,
    #[serde(rename = "marketCap")]
    #[serde(default)]
    market_cap: WireValue<RawDecimal>,
    #[serde(rename = "trailingPE")]
    #[serde(default)]
    trailing_pe: WireValue<RawDecimal>,
    #[serde(rename = "dividendRate")]
    #[serde(default)]
    dividend_rate: WireValue<RawDecimal>,
    #[serde(rename = "dividendYield")]
    #[serde(default)]
    dividend_yield: WireValue<RawDecimal>,
    #[serde(rename = "trailingAnnualDividendYield")]
    #[serde(default)]
    trailing_annual_dividend_yield: WireValue<RawDecimal>,
    #[serde(rename = "exDividendDate")]
    #[serde(default)]
    ex_dividend_date: WireValue<RawDate>,
    #[serde(rename = "fiftyDayAverage")]
    #[serde(default)]
    fifty_day_average: WireValue<RawDecimal>,
    #[serde(rename = "twoHundredDayAverage")]
    #[serde(default)]
    two_hundred_day_average: WireValue<RawDecimal>,
    #[serde(rename = "fiftyTwoWeekHigh")]
    #[serde(default)]
    fifty_two_week_high: WireValue<RawDecimal>,
    #[serde(rename = "fiftyTwoWeekLow")]
    #[serde(default)]
    fifty_two_week_low: WireValue<RawDecimal>,
    #[serde(rename = "averageVolume")]
    #[serde(default)]
    average_volume: WireValue<RawNumU64>,
}

#[derive(Default, Deserialize)]
struct DefaultKeyStatisticsNode {
    #[serde(default)]
    beta: WireValue<RawDecimal>,
    #[serde(rename = "sharesOutstanding")]
    #[serde(default)]
    shares_outstanding: WireValue<RawNumU64>,
    #[serde(rename = "trailingEps")]
    #[serde(default)]
    trailing_eps: WireValue<RawDecimal>,
}

struct QuoteSummaryKeyStatisticsFields {
    summary_currency: Option<String>,
    beta: Option<Decimal>,
    fifty_day_average: Option<Decimal>,
    two_hundred_day_average: Option<Decimal>,
    market_cap: Option<Decimal>,
    shares_outstanding: Option<u64>,
    trailing_eps: Option<Decimal>,
    trailing_pe: Option<Decimal>,
    dividend_rate: Option<Decimal>,
    trailing_annual_dividend_yield: Option<Decimal>,
    dividend_yield: Option<Decimal>,
    ex_dividend_date: Option<i64>,
    fifty_two_week_high: Option<Decimal>,
    fifty_two_week_low: Option<Decimal>,
    average_volume: Option<u64>,
}

fn quote_summary_key_statistics_fields(
    ctx: &mut ProjectionContext,
    key: Option<&str>,
    summary_detail: &SummaryDetailNode,
    default_key_statistics: &DefaultKeyStatisticsNode,
) -> Result<QuoteSummaryKeyStatisticsFields, YfError> {
    let summary_beta =
        summary_detail
            .beta
            .optional_copied_and_then(ctx, "summaryDetail.beta", key, |raw| raw.raw)?;
    let default_beta = default_key_statistics.beta.optional_copied_and_then(
        ctx,
        "defaultKeyStatistics.beta",
        key,
        |raw| raw.raw,
    )?;

    Ok(QuoteSummaryKeyStatisticsFields {
        summary_currency: summary_detail.currency.optional_cloned(
            ctx,
            "summaryDetail.currency",
            key,
        )?,
        beta: summary_beta.or(default_beta),
        fifty_day_average: summary_detail.fifty_day_average.optional_copied_and_then(
            ctx,
            "summaryDetail.fiftyDayAverage",
            key,
            |raw| raw.raw,
        )?,
        two_hundred_day_average: summary_detail
            .two_hundred_day_average
            .optional_copied_and_then(ctx, "summaryDetail.twoHundredDayAverage", key, |raw| {
                raw.raw
            })?,
        market_cap: summary_detail.market_cap.optional_copied_and_then(
            ctx,
            "summaryDetail.marketCap",
            key,
            |raw| raw.raw,
        )?,
        shares_outstanding: default_key_statistics
            .shares_outstanding
            .optional_copied_and_then(
                ctx,
                "defaultKeyStatistics.sharesOutstanding",
                key,
                |raw| raw.raw,
            )?,
        trailing_eps: default_key_statistics
            .trailing_eps
            .optional_copied_and_then(ctx, "defaultKeyStatistics.trailingEps", key, |raw| {
                raw.raw
            })?,
        trailing_pe: summary_detail.trailing_pe.optional_copied_and_then(
            ctx,
            "summaryDetail.trailingPE",
            key,
            |raw| raw.raw,
        )?,
        dividend_rate: summary_detail.dividend_rate.optional_copied_and_then(
            ctx,
            "summaryDetail.dividendRate",
            key,
            |raw| raw.raw,
        )?,
        trailing_annual_dividend_yield: summary_detail
            .trailing_annual_dividend_yield
            .optional_copied_and_then(
                ctx,
                "summaryDetail.trailingAnnualDividendYield",
                key,
                |raw| raw.raw,
            )?,
        dividend_yield: summary_detail.dividend_yield.optional_copied_and_then(
            ctx,
            "summaryDetail.dividendYield",
            key,
            |raw| raw.raw,
        )?,
        ex_dividend_date: summary_detail.ex_dividend_date.optional_copied_and_then(
            ctx,
            "summaryDetail.exDividendDate",
            key,
            |raw| raw.raw,
        )?,
        fifty_two_week_high: summary_detail
            .fifty_two_week_high
            .optional_copied_and_then(ctx, "summaryDetail.fiftyTwoWeekHigh", key, |raw| raw.raw)?,
        fifty_two_week_low: summary_detail.fifty_two_week_low.optional_copied_and_then(
            ctx,
            "summaryDetail.fiftyTwoWeekLow",
            key,
            |raw| raw.raw,
        )?,
        average_volume: summary_detail.average_volume.optional_copied_and_then(
            ctx,
            "summaryDetail.averageVolume",
            key,
            |raw| raw.raw,
        )?,
    })
}

impl QuoteSummaryKeyStatistics {
    pub fn into_key_statistics_and_moving_averages_with_context(
        self,
        ctx: &mut ProjectionContext,
        symbol: &str,
    ) -> Result<(KeyStatistics, MovingAverages), YfError> {
        let key = Some(symbol);
        let summary_detail = self.summary_detail.unwrap_or_default();
        let default_key_statistics = self.default_key_statistics.unwrap_or_default();
        let fields = quote_summary_key_statistics_fields(
            ctx,
            key,
            &summary_detail,
            &default_key_statistics,
        )?;
        let currencies =
            QuoteCurrencyUnits::from_quote_summary_currency(fields.summary_currency.as_deref());
        let moving_averages = MovingAverages {
            fifty_day: currencies.quote_price(
                ctx,
                "summaryDetail.fiftyDayAverage",
                key,
                fields.fifty_day_average,
                "50-day moving average",
            )?,
            two_hundred_day: currencies.quote_price(
                ctx,
                "summaryDetail.twoHundredDayAverage",
                key,
                fields.two_hundred_day_average,
                "200-day moving average",
            )?,
        };

        let key_statistics = KeyStatistics {
            market_cap: currencies.quote_money(
                ctx,
                "summaryDetail.marketCap",
                key,
                fields.market_cap,
                "market cap",
            )?,
            shares_outstanding: fields.shares_outstanding,
            eps_trailing_twelve_months: currencies.financial_price(
                ctx,
                "defaultKeyStatistics.trailingEps",
                key,
                fields.trailing_eps,
                "trailing EPS",
            )?,
            pe_trailing_twelve_months: fields.trailing_pe,
            dividend_per_share_forward: currencies.quote_major_price(
                ctx,
                "summaryDetail.dividendRate",
                key,
                fields.dividend_rate,
                "forward dividend per share",
            )?,
            dividend_yield_trailing: fields.trailing_annual_dividend_yield,
            dividend_yield_forward: fields.dividend_yield,
            ex_dividend_date: optional_date(
                ctx,
                "summaryDetail.exDividendDate",
                key,
                fields.ex_dividend_date,
            )?,
            fifty_two_week_high: currencies.quote_price(
                ctx,
                "summaryDetail.fiftyTwoWeekHigh",
                key,
                fields.fifty_two_week_high,
                "52-week high",
            )?,
            fifty_two_week_low: currencies.quote_price(
                ctx,
                "summaryDetail.fiftyTwoWeekLow",
                key,
                fields.fifty_two_week_low,
                "52-week low",
            )?,
            average_daily_volume_3m: fields.average_volume,
            beta: fields.beta,
            as_of: None,
        };

        Ok((key_statistics, moving_averages))
    }

    pub fn into_key_statistics_with_context(
        self,
        ctx: &mut ProjectionContext,
        symbol: &str,
    ) -> Result<KeyStatistics, YfError> {
        Ok(self
            .into_key_statistics_and_moving_averages_with_context(ctx, symbol)?
            .0)
    }
}

fn optional_date(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    timestamp: Option<i64>,
) -> Result<Option<chrono::NaiveDate>, YfError> {
    let Some(timestamp) = timestamp else {
        return Ok(None);
    };

    match i64_to_date(timestamp) {
        Ok(date) => Ok(Some(date)),
        Err(err) => {
            ctx.omitted_present_field(
                path,
                key,
                ProjectionIssue::InvalidField {
                    field: path,
                    details: err.to_string(),
                },
            )?;
            Ok(None)
        }
    }
}

pub fn quote_summary_key_statistics_from_raw(
    raw: &serde_json::value::RawValue,
) -> Result<QuoteSummaryKeyStatistics, YfError> {
    serde_json::from_str(raw.get()).map_err(YfError::json)
}

pub fn merge_key_statistics(
    mut base: KeyStatistics,
    quote_summary: &KeyStatistics,
) -> KeyStatistics {
    if base.market_cap.is_none() {
        base.market_cap.clone_from(&quote_summary.market_cap);
    }
    if base.shares_outstanding.is_none() {
        base.shares_outstanding = quote_summary.shares_outstanding;
    }
    if base.eps_trailing_twelve_months.is_none() {
        base.eps_trailing_twelve_months
            .clone_from(&quote_summary.eps_trailing_twelve_months);
    }
    if base.pe_trailing_twelve_months.is_none() {
        base.pe_trailing_twelve_months = quote_summary.pe_trailing_twelve_months;
    }
    if base.dividend_per_share_forward.is_none() {
        base.dividend_per_share_forward
            .clone_from(&quote_summary.dividend_per_share_forward);
    }
    if base.dividend_yield_trailing.is_none() {
        base.dividend_yield_trailing = quote_summary.dividend_yield_trailing;
    }
    if base.dividend_yield_forward.is_none() {
        base.dividend_yield_forward = quote_summary.dividend_yield_forward;
    }
    if base.ex_dividend_date.is_none() {
        base.ex_dividend_date = quote_summary.ex_dividend_date;
    }
    if base.fifty_two_week_high.is_none() {
        base.fifty_two_week_high
            .clone_from(&quote_summary.fifty_two_week_high);
    }
    if base.fifty_two_week_low.is_none() {
        base.fifty_two_week_low
            .clone_from(&quote_summary.fifty_two_week_low);
    }
    if base.average_daily_volume_3m.is_none() {
        base.average_daily_volume_3m = quote_summary.average_daily_volume_3m;
    }
    if base.beta.is_none() {
        base.beta = quote_summary.beta;
    }
    base
}

pub fn merge_moving_averages(
    mut base: MovingAverages,
    quote_summary: &MovingAverages,
) -> MovingAverages {
    if base.fifty_day.is_none() {
        base.fifty_day.clone_from(&quote_summary.fifty_day);
    }
    if base.two_hundred_day.is_none() {
        base.two_hundred_day
            .clone_from(&quote_summary.two_hundred_day);
    }
    base
}

pub async fn fetch_quote_summary_key_statistics(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<QuoteSummaryKeyStatistics, YfError> {
    let body = quotesummary::fetch_body(
        client,
        symbol,
        KEY_STATISTICS_MODULES,
        "key_statistics",
        options,
    )
    .await?;

    quotesummary::parse_module_result(&body)
}

/// Centralized function to fetch one or more quotes from the v7 API.
/// It handles caching, retries, and authentication (crumb).
pub async fn fetch_v7_quotes(
    client: &YfClient,
    symbols: &[&str],
    options: &CallOptions,
) -> Result<Vec<V7QuoteNode>, YfError> {
    let values = fetch_v7_quote_values(client, symbols, options).await?;
    let mut ctx = ProjectionContext::new("quote_v7", options.data_quality());
    report_missing_requested_quote_values(symbols, &values, &mut ctx)?;
    quote_nodes_from_values_with_context(client, symbols, values, &mut ctx)
}

pub fn report_missing_requested_quote_values(
    requested_symbols: &[&str],
    values: &[Value],
    ctx: &mut ProjectionContext,
) -> Result<(), YfError> {
    let normalized_symbols = normalize_symbols(requested_symbols.iter().copied())?;
    let requested_symbols = normalized_symbols
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut resolved_requests = vec![false; requested_symbols.len()];

    for value in values {
        let provider_symbol = value
            .get("symbol")
            .and_then(serde_json::Value::as_str)
            .and_then(|symbol| nonempty_symbol(Some(symbol)));
        mark_resolved_requested_symbol(&requested_symbols, &mut resolved_requests, provider_symbol);
    }

    for (symbol, resolved) in requested_symbols.iter().zip(resolved_requests) {
        if !resolved {
            ctx.dropped_item(
                "quote",
                Some(*symbol),
                ProjectionIssue::ProviderUnavailable { feature: "quote" },
            )?;
        }
    }

    Ok(())
}

/// Centralized function to fetch raw quote values from the v7 API.
/// Projection callers can then choose strict or best-effort node conversion.
pub async fn fetch_v7_quote_values(
    client: &YfClient,
    symbols: &[&str],
    options: &CallOptions,
) -> Result<Vec<Value>, YfError> {
    if symbols.is_empty() {
        return Err(YfError::InvalidParams(
            "symbols list cannot be empty".into(),
        ));
    }

    let normalized_symbols = normalize_symbols(symbols.iter().copied())?;
    let chunks = chunk_v7_quote_symbols(client.base_quote_v7(), normalized_symbols)?;

    let mut chunk_values: Vec<_> = stream::iter(chunks.into_iter().enumerate())
        .map(|(index, symbols)| async move {
            let values = fetch_v7_quote_chunk(client, &symbols, options).await?;
            Ok::<_, YfError>((index, values))
        })
        .buffer_unordered(MAX_V7_QUOTE_CONCURRENT_REQUESTS)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<_, _>>()?;

    chunk_values.sort_unstable_by_key(|(index, _)| *index);
    let total_len = chunk_values.iter().map(|(_, values)| values.len()).sum();
    let mut values = Vec::with_capacity(total_len);
    for (_, mut chunk) in chunk_values {
        values.append(&mut chunk);
    }

    Ok(values)
}

fn chunk_v7_quote_symbols(
    base_url: &Url,
    normalized_symbols: Vec<String>,
) -> Result<Vec<Vec<String>>, YfError> {
    let mut chunks = Vec::new();
    let mut current = Vec::new();

    for symbol in normalized_symbols {
        let mut candidate = current.clone();
        candidate.push(symbol.clone());

        if candidate.len() > MAX_V7_QUOTE_SYMBOLS_PER_REQUEST
            || v7_quote_url(&candidate, base_url).as_str().len() > MAX_V7_QUOTE_URL_BYTES
        {
            if current.is_empty() {
                return Err(YfError::InvalidParams(format!(
                    "symbol {symbol:?} makes v7 quote URL exceed {MAX_V7_QUOTE_URL_BYTES} bytes"
                )));
            }

            chunks.push(std::mem::take(&mut current));
            current.push(symbol);
        } else {
            current = candidate;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    Ok(chunks)
}

async fn fetch_v7_quote_chunk(
    client: &YfClient,
    normalized_symbols: &[String],
    options: &CallOptions,
) -> Result<Vec<Value>, YfError> {
    let url = v7_quote_url(normalized_symbols, client.base_quote_v7());
    let fixture_key = normalized_symbols.join("-");

    let (body_to_parse, _) = net::fetch_text_with_auth_retry(
        client,
        url,
        net::AuthFetchConfig {
            auth_mode: net::AuthMode::OptionalCrumb,
            cache_endpoint: CacheEndpoint::Quote,
            options,
            cache_body: None,
            endpoint: "quote_v7",
            fixture_key: &fixture_key,
            ext: "json",
            retry_on_invalid_crumb_body: true,
            cache_validator: Some(validate_v7_quote_body),
        },
        |url| client.http().get(url).header("accept", "application/json"),
    )
    .await?;

    let env: V7Envelope = serde_json::from_str(&body_to_parse).map_err(YfError::json)?;
    let quote_response = env
        .quote_response
        .ok_or_else(|| YfError::MissingData("v7 quoteResponse missing".into()))?;
    reject_v7_quote_error(&quote_response)?;

    let nodes = quote_response
        .result
        .ok_or_else(|| YfError::MissingData("v7 quoteResponse.result missing".into()))?;

    Ok(nodes)
}

fn v7_quote_url(normalized_symbols: &[String], base_url: &Url) -> Url {
    let mut url = base_url.clone();
    url.query_pairs_mut()
        .append_pair("symbols", &normalized_symbols.join(","));
    url
}

fn validate_v7_quote_body(body: &str) -> Result<(), YfError> {
    let env: V7Envelope = serde_json::from_str(body).map_err(YfError::json)?;
    let quote_response = env
        .quote_response
        .ok_or_else(|| YfError::MissingData("v7 quoteResponse missing".into()))?;
    reject_v7_quote_error(&quote_response)
}

fn reject_v7_quote_error(quote_response: &V7QuoteResponse) -> Result<(), YfError> {
    if let Some(error) = quote_response.error.as_ref() {
        crate::core::logging::trace_error!(
            description = %error.description,
            "quoteResponse error"
        );
        return Err(YfError::Api(format!("yahoo error: {}", error.description)));
    }

    Ok(())
}

pub fn quote_nodes_from_values_with_context(
    client: &YfClient,
    requested_symbols: &[&str],
    values: Vec<Value>,
    ctx: &mut ProjectionContext,
) -> Result<Vec<V7QuoteNode>, YfError> {
    let mut nodes = Vec::with_capacity(values.len());
    for (idx, value) in values.into_iter().enumerate() {
        if let Some(node) = quote_node_from_value_with_context(value, idx, ctx)? {
            nodes.push(node);
        }
    }
    store_v7_quote_side_effects(client, requested_symbols, &nodes);
    Ok(nodes)
}

fn quote_node_from_value_with_context(
    value: Value,
    idx: usize,
    ctx: &mut ProjectionContext,
) -> Result<Option<V7QuoteNode>, YfError> {
    let key = Some(quote_node_diag_key_from_value(&value, idx));
    match serde_json::from_value(value) {
        Ok(node) => Ok(Some(node)),
        Err(err) => {
            ctx.dropped_item(
                "quote",
                key.as_deref(),
                ProjectionIssue::InvalidField {
                    field: "quote",
                    details: err.to_string(),
                },
            )?;
            Ok(None)
        }
    }
}

pub fn first_quote_node_from_nodes(nodes: Vec<V7QuoteNode>) -> Option<V7QuoteNode> {
    nodes.into_iter().next()
}

pub fn required_quote_node_from_nodes(
    nodes: Vec<V7QuoteNode>,
    symbol: &str,
) -> Result<V7QuoteNode, YfError> {
    first_quote_node_from_nodes(nodes).ok_or_else(|| {
        YfError::MissingData(format!("no valid quote result found for symbol {symbol}"))
    })
}

fn quote_node_diag_key_from_value(value: &Value, idx: usize) -> String {
    value
        .get("symbol")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map_or_else(|| format!("result[{idx}]"), ToString::to_string)
}

fn store_v7_quote_side_effects(
    client: &YfClient,
    requested_symbols: &[&str],
    nodes: &[V7QuoteNode],
) {
    let mut resolved_requests = vec![false; requested_symbols.len()];

    for node in nodes {
        let provider_symbol = nonempty_symbol(node.symbol.as_str());
        if let Some(symbol) = provider_symbol {
            store_quote_node_hints(client, symbol, node);
            store_requested_alias_hints(client, requested_symbols, symbol, node);
            store_quote_node_instrument(client, symbol, node);
        }

        mark_resolved_requested_symbol(requested_symbols, &mut resolved_requests, provider_symbol);

        if requested_symbols.len() == 1
            && provider_symbol.is_none_or(|symbol| !same_symbol(symbol, requested_symbols[0]))
        {
            store_quote_node_hints(client, requested_symbols[0], node);
        }
    }

    for (symbol, resolved) in requested_symbols.iter().zip(resolved_requests) {
        if !resolved {
            client.store_currency_hints(
                symbol,
                CurrencyHints::from_quote(None, None, None, None, None),
            );
        }
    }
}

fn store_quote_node_hints(client: &YfClient, symbol: &str, node: &V7QuoteNode) {
    client.store_currency_hints(
        symbol,
        CurrencyHints::from_quote(
            node.currency.as_str(),
            node.financial_currency.as_str(),
            node.exchange.as_str(),
            node.full_exchange_name.as_str(),
            node.quote_type.as_str(),
        ),
    );
}

fn store_quote_node_instrument(client: &YfClient, symbol: &str, node: &V7QuoteNode) {
    let exch = node.exchange();
    let Some(kind) = node
        .quote_type
        .as_str()
        .and_then(|s| parse_yahoo_quote_type(s).ok())
    else {
        return;
    };

    let inst = match exch {
        Some(ex) => Instrument::from_symbol_and_exchange(symbol, ex, kind),
        None => Instrument::from_symbol(symbol, kind),
    };
    if let Ok(inst) = inst {
        client.store_instrument(symbol.to_string(), inst);
    }
}

fn store_requested_alias_hints(
    client: &YfClient,
    requested_symbols: &[&str],
    provider_symbol: &str,
    node: &V7QuoteNode,
) {
    for requested in requested_symbols {
        if same_symbol(provider_symbol, requested) && !same_cache_key(provider_symbol, requested) {
            store_quote_node_hints(client, requested, node);
        }
    }
}

fn mark_resolved_requested_symbol(
    requested_symbols: &[&str],
    resolved: &mut [bool],
    provider_symbol: Option<&str>,
) {
    if let Some(symbol) = provider_symbol {
        for (idx, requested) in requested_symbols.iter().enumerate() {
            if same_symbol(symbol, requested) {
                resolved[idx] = true;
            }
        }
    }

    if requested_symbols.len() == 1
        && provider_symbol.is_none_or(|symbol| !same_symbol(symbol, requested_symbols[0]))
    {
        resolved[0] = true;
    }
}

fn nonempty_symbol(symbol: Option<&str>) -> Option<&str> {
    symbol.map(str::trim).filter(|symbol| !symbol.is_empty())
}

fn same_symbol(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn same_cache_key(left: &str, right: &str) -> bool {
    left.trim() == right.trim()
}

impl TryFrom<V7QuoteNode> for Quote {
    type Error = YfError;

    fn try_from(n: V7QuoteNode) -> Result<Self, Self::Error> {
        let mut ctx = ProjectionContext::new("quote_v7", DataQuality::BestEffort);
        n.to_quote_with_context(&mut ctx)
    }
}

impl V7QuoteNode {
    pub(crate) fn to_quote_item_with_context(
        &self,
        ctx: &mut ProjectionContext,
    ) -> Result<Option<Quote>, YfError> {
        let key = self.symbol_key();
        let exchange = self.exchange_with_context(ctx, key.as_deref())?;
        let instrument = match self.instrument_projection(exchange) {
            Ok(instrument) => instrument,
            Err(reason) => {
                ctx.dropped_item("quote", key.as_deref(), reason)?;
                return Ok(None);
            }
        };
        let currencies = self.currency_units();
        let currency = match currencies.quote_currency() {
            Ok(currency) => currency,
            Err(reason) => {
                ctx.dropped_item("quote", key.as_deref(), reason)?;
                return Ok(None);
            }
        };

        self.quote_from_instrument_with_context(ctx, key.as_deref(), instrument, currency)
            .map(Some)
    }

    pub(crate) fn to_quote_with_context(
        &self,
        ctx: &mut ProjectionContext,
    ) -> Result<Quote, YfError> {
        let key = self.symbol_key();
        let exchange = self.exchange_with_context(ctx, key.as_deref())?;
        let instrument = self.instrument(exchange)?;
        let currency = self
            .currency_units()
            .quote_currency()
            .map_err(|issue| YfError::InvalidData(format!("invalid quote currency: {issue}")))?;

        self.quote_from_instrument_with_context(ctx, key.as_deref(), instrument, currency)
    }

    fn quote_from_instrument_with_context(
        &self,
        ctx: &mut ProjectionContext,
        key: Option<&str>,
        instrument: Instrument,
        currency: Currency,
    ) -> Result<Quote, YfError> {
        let currencies = self.currency_units();
        let name = self
            .long_name
            .optional_cloned(ctx, "longName", key)?
            .or(self.short_name.optional_cloned(ctx, "shortName", key)?);
        let regular_market_price = self.regular_market_price.optional_copied_map(
            ctx,
            "regularMarketPrice",
            key,
            JsonDecimal::into_decimal,
        )?;
        let bid = self
            .bid
            .optional_copied_map(ctx, "bid", key, JsonDecimal::into_decimal)?;
        let bid_size = self
            .bid_size
            .optional_copied_map(ctx, "bidSize", key, JsonU64::into_u64)?;
        let ask = self
            .ask
            .optional_copied_map(ctx, "ask", key, JsonDecimal::into_decimal)?;
        let ask_size = self
            .ask_size
            .optional_copied_map(ctx, "askSize", key, JsonU64::into_u64)?;
        let regular_market_previous_close =
            self.regular_market_previous_close.optional_copied_map(
                ctx,
                "regularMarketPreviousClose",
                key,
                JsonDecimal::into_decimal,
            )?;
        let regular_market_volume = self.regular_market_volume.optional_copied_map(
            ctx,
            "regularMarketVolume",
            key,
            JsonU64::into_u64,
        )?;

        Ok(Quote {
            instrument,
            name,
            currency,
            price: currencies.quote_price_amount(
                ctx,
                "regularMarketPrice",
                key,
                regular_market_price,
                "quote price",
            )?,
            bid: self.positive_book_level(ctx, "bid", key, bid, bid_size)?,
            ask: self.positive_book_level(ctx, "ask", key, ask, ask_size)?,
            previous_close: currencies.quote_price_amount(
                ctx,
                "regularMarketPreviousClose",
                key,
                regular_market_previous_close,
                "quote previous close",
            )?,
            day_volume: regular_market_volume.and_then(quantity_from_u64),
            market_state: self.market_state_with_context(ctx, key)?,
            as_of: self.as_of_with_context(ctx, key)?,
            provider: (),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_base_url() -> Url {
        Url::parse("https://query1.finance.yahoo.com/v7/finance/quote").unwrap()
    }

    #[test]
    fn quote_symbol_chunks_respect_max_symbol_count() {
        let symbols = (0..=MAX_V7_QUOTE_SYMBOLS_PER_REQUEST)
            .map(|idx| format!("SYM{idx}"))
            .collect::<Vec<_>>();

        let chunks = chunk_v7_quote_symbols(&test_base_url(), symbols).unwrap();

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), MAX_V7_QUOTE_SYMBOLS_PER_REQUEST);
        assert_eq!(chunks[1].len(), 1);
    }

    #[test]
    fn quote_symbol_chunks_respect_encoded_url_byte_limit() {
        let symbols = (0..40)
            .map(|idx| format!("SYM{idx:03}{}", "X".repeat(70)))
            .collect::<Vec<_>>();

        let chunks = chunk_v7_quote_symbols(&test_base_url(), symbols).unwrap();

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| {
            v7_quote_url(chunk, &test_base_url()).as_str().len() <= MAX_V7_QUOTE_URL_BYTES
        }));
    }
}
