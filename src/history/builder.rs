mod actions;
mod adjust;
mod assemble;
mod fetch;

use crate::core::yahoo_vocab::{first_parsed_yahoo_exchange, parse_yahoo_quote_type};
use crate::core::{
    CallOptions, DataQuality, ProjectionContext, ProjectionIssue, YfClient, YfError, YfResponse,
    wire::{JsonDecimal, WireField, WireValue},
};
use crate::core::{
    client::normalize_symbol,
    currency_resolver::{
        CorporateActionCurrencyEvidence, CurrencyHints, CurrencyPurpose, ResolvedCurrencyUnit,
        TradingCurrencyEvidence, project_currency_resolution,
    },
};
use crate::history::wire::MetaNode;
use crate::history::{YahooHistoryResponse, round_candle_prices};
use chrono_tz::Tz;
use paft::domain::Instrument;
use paft::market::action::Action;
use paft::market::requests::history::{Interval, Range};
use paft::market::responses::history::{
    Candle, HistoryMeta, HistoryResponse, OhlcPriceBasis, PriceBasis,
};

use actions::extract_actions;
use adjust::{AdjustmentBasis, AdjustmentPlan, SharedAdjustmentFactor, split_adjustments_after};
use assemble::{adjustment_plan_for_series, assemble_candles};
use fetch::{ChartFetchRequest, fetch_chart};

/// A builder for fetching historical price data for a single symbol.
///
/// This builder provides fine-grained control over the parameters for a historical
/// data request, including the time range, interval, and data adjustments.
#[derive(Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct HistoryBuilder {
    #[doc(hidden)]
    pub(crate) client: YfClient,
    #[doc(hidden)]
    pub(crate) symbol: String,
    #[doc(hidden)]
    pub(crate) range: Option<Range>,
    #[doc(hidden)]
    pub(crate) period: Option<(i64, i64)>,
    #[doc(hidden)]
    pub(crate) interval: Interval,
    #[doc(hidden)]
    pub(crate) auto_adjust: bool,
    #[doc(hidden)]
    pub(crate) rounding: bool,
    #[doc(hidden)]
    pub(crate) include_prepost: bool,
    #[doc(hidden)]
    pub(crate) include_actions: bool,
    #[doc(hidden)]
    options: CallOptions,
}

impl HistoryBuilder {
    /// Creates a new `HistoryBuilder` for a given symbol.
    pub fn new(client: &YfClient, symbol: impl Into<String>) -> Self {
        Self {
            client: client.clone(),
            symbol: symbol.into(),
            range: Some(Range::M6),
            period: None,
            interval: Interval::D1,
            auto_adjust: true,
            rounding: false,
            include_prepost: false,
            include_actions: true,
            options: CallOptions::default(),
        }
    }

    crate::core::impl_call_option_setters!();

    /// Sets a relative time range for the request (e.g., `1y`, `6mo`).
    ///
    /// This will override any previously set period using `between()`.
    #[must_use]
    pub const fn range(mut self, range: Range) -> Self {
        self.period = None;
        self.range = Some(range);
        self
    }

    /// Sets an absolute time period for the request using start and end timestamps.
    ///
    /// This will override any previously set range using `range()`.
    #[must_use]
    pub const fn between(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.range = None;
        self.period = Some((start.timestamp(), end.timestamp()));
        self
    }

    /// Sets the time interval for each data point (candle).
    #[must_use]
    pub const fn interval(mut self, interval: Interval) -> Self {
        self.interval = interval;
        self
    }

    /// Sets whether to automatically adjust prices for splits and dividends. (Default: `true`)
    #[must_use]
    pub const fn auto_adjust(mut self, yes: bool) -> Self {
        self.auto_adjust = yes;
        self
    }

    /// Sets whether to round candle prices using Yahoo's chart `priceHint`. (Default: `false`)
    ///
    /// Rounding is applied after price adjustment. Without it, no display rounding is applied to
    /// provider or computed candle values. Corporate-action amounts are always left exact.
    #[must_use]
    pub const fn rounding(mut self, yes: bool) -> Self {
        self.rounding = yes;
        self
    }

    /// Sets whether to include pre-market and post-market data for intraday intervals. (Default: `false`)
    #[must_use]
    pub const fn prepost(mut self, yes: bool) -> Self {
        self.include_prepost = yes;
        self
    }

    /// Sets whether to include corporate actions (dividends and splits) in the response. (Default: `true`)
    #[must_use]
    pub const fn actions(mut self, yes: bool) -> Self {
        self.include_actions = yes;
        self
    }

    /// Executes the request and returns only the price candles.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails or the API response cannot be parsed.
    pub async fn fetch(&self) -> Result<Vec<Candle>, YfError> {
        Ok(self.fetch_with_diagnostics().await?.into_data())
    }

    /// Executes the request and returns candles with projection diagnostics.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails, the API returns an error,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn fetch_with_diagnostics(&self) -> Result<YfResponse<Vec<Candle>>, YfError> {
        Ok(self
            .fetch_full_with_diagnostics()
            .await?
            .map(|resp| resp.candles))
    }

    /// Executes the request and returns the full response, including candles, actions, and metadata.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails, the API returns an error,
    /// or the response cannot be parsed.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            skip(self),
            err,
            fields(
                symbol = %self.symbol,
                interval = %format!("{:?}", self.interval),
                range = %self
                    .range
                    .as_ref()
                    .map_or_else(|| "period".into(), |r| format!("{r:?}"))
            )
        )
    )]
    pub async fn fetch_full(&self) -> Result<HistoryResponse, YfError> {
        Ok(self.fetch_full_with_diagnostics().await?.into_data())
    }

    /// Executes the request and returns the full response with projection diagnostics.
    ///
    /// # Errors
    ///
    /// Returns a `YfError` if the network request fails, the API returns an error,
    /// or strict data-quality mode rejects a projection issue.
    pub async fn fetch_full_with_diagnostics(
        &self,
    ) -> Result<YfResponse<HistoryResponse>, YfError> {
        Ok(self
            .fetch_full_yahoo_with_diagnostics()
            .await?
            .map(|history| history.response))
    }

    pub(crate) async fn fetch_full_yahoo_with_diagnostics(
        &self,
    ) -> Result<YfResponse<YahooHistoryResponse>, YfError> {
        let mut ctx = ProjectionContext::new("history_chart", self.options.data_quality());
        let symbol = normalize_symbol(&self.symbol)?;

        // 1) Fetch and parse the /chart payload into owned blocks
        let fetched = fetch_chart(
            &self.client,
            &symbol,
            ChartFetchRequest {
                range: self.range,
                period: self.period,
                interval: self.interval,
                include_actions: self.include_actions,
                include_prepost: self.include_prepost,
            },
            &self.options,
        )
        .await?;

        let instrument = store_history_side_effects(&self.client, &symbol, fetched.meta.as_ref());

        // 2) Corporate actions & split ratios
        let chart_currency = fetched.meta.as_ref().and_then(|m| m.currency.as_deref());
        let has_complete_candle = has_complete_ohlc_row(&fetched.quote, fetched.ts.len());
        let currency = if has_complete_candle {
            history_trading_currency(
                &self.client,
                &symbol,
                chart_currency,
                &self.options,
                &mut ctx,
            )
            .await?
        } else {
            None
        };
        let action_currency = action_default_currency(
            &self.client,
            &symbol,
            fetched.events.as_ref(),
            chart_currency,
            currency.as_ref(),
            &self.options,
            &mut ctx,
        )
        .await?;

        let (mut actions_out, split_events) =
            extract_actions(fetched.events.as_ref(), action_currency.as_ref(), &mut ctx)?;

        // 3) Exact rational split adjustments after each bar
        let split_adjustments = split_adjustments_after(&fetched.ts, &split_events);

        // 4) Assemble candles (+ raw close) with/without adjustments
        let mut adjustment_basis = None;
        let mut candles = if let Some(currency) = currency.as_ref() {
            let adjustment_plan = history_adjustment_plan(
                self.auto_adjust,
                &fetched.quote,
                &fetched.adjclose,
                fetched.ts.len(),
                &mut ctx,
            )?;
            adjustment_basis = adjustment_plan.as_ref().map(AdjustmentPlan::basis);
            assemble_candles(
                &fetched.ts,
                &fetched.quote,
                adjustment_plan.as_ref(),
                &split_adjustments,
                currency,
                &mut ctx,
            )?
        } else {
            if !has_complete_candle && has_any_ohlc_value(&fetched.quote, fetched.ts.len()) {
                ctx.dropped_item(
                    "candle",
                    None,
                    ProjectionIssue::MissingRequiredFields {
                        fields: vec!["open", "high", "low", "close"],
                    },
                )?;
            }
            Vec::new()
        };

        // ensure actions sorted (extract_actions already sorts, keep consistent)
        actions_out.sort_by_key(action_sort_key);

        // 5) Map metadata
        let meta_out = map_meta(fetched.meta.as_ref(), &mut ctx)?;
        let price_hint = fetched
            .meta
            .as_ref()
            .and_then(|meta| u32::try_from(meta.price_hint?).ok());
        if self.rounding
            && let Some(price_hint) = price_hint
        {
            round_candle_prices(&mut candles, price_hint, currency.as_ref());
        }
        let price_basis = history_price_basis(adjustment_basis, &split_adjustments);

        Ok(ctx.finish(YahooHistoryResponse {
            response: HistoryResponse {
                candles,
                actions: actions_out,
                price_basis,
                meta: meta_out,
                provider: (),
            },
            price_hint,
            currency_unit: currency,
            instrument,
        }))
    }
}

/* --- tiny private helper --- */

fn has_complete_ohlc_row(quote: &crate::history::wire::QuoteBlock, len: usize) -> bool {
    (0..len).any(|idx| {
        quote.open.get(idx).and_then(WireValue::as_ref).is_some()
            && quote.high.get(idx).and_then(WireValue::as_ref).is_some()
            && quote.low.get(idx).and_then(WireValue::as_ref).is_some()
            && quote.close.get(idx).and_then(WireValue::as_ref).is_some()
    })
}

fn has_any_ohlc_value(quote: &crate::history::wire::QuoteBlock, len: usize) -> bool {
    (0..len).any(|idx| {
        field_is_present(quote.open.get(idx))
            || field_is_present(quote.high.get(idx))
            || field_is_present(quote.low.get(idx))
            || field_is_present(quote.close.get(idx))
    })
}

fn field_is_present(field: Option<&WireValue<JsonDecimal>>) -> bool {
    field.is_some_and(|field| field.as_ref().is_some() || field.invalid_details().is_some())
}

const fn action_sort_key(action: &Action) -> (bool, chrono::NaiveDate) {
    match action {
        Action::Dividend { date, .. }
        | Action::Split { date, .. }
        | Action::CapitalGain { date, .. } => (false, *date),
        _ => (true, chrono::NaiveDate::MAX),
    }
}

fn history_price_basis(
    adjustment_basis: Option<AdjustmentBasis>,
    split_adjustments: &[Option<SharedAdjustmentFactor>],
) -> OhlcPriceBasis {
    match adjustment_basis {
        Some(AdjustmentBasis::ProviderAdjusted) => {
            OhlcPriceBasis::uniform(PriceBasis::provider_latest_adjusted())
        }
        Some(AdjustmentBasis::SplitAdjusted)
            if split_adjustments
                .iter()
                .flatten()
                .any(|factor| !factor.is_identity()) =>
        {
            OhlcPriceBasis::uniform(PriceBasis::split_adjusted_latest())
        }
        None | Some(AdjustmentBasis::SplitAdjusted) => OhlcPriceBasis::raw(),
    }
}

fn store_history_side_effects(
    client: &YfClient,
    symbol: &str,
    meta: Option<&MetaNode>,
) -> Option<Instrument> {
    let instrument = history_instrument_from_meta(symbol, meta);
    if let (Some(meta), Some(instrument)) = (meta, instrument.as_ref()) {
        store_history_instrument(client, symbol, meta, instrument);
    }
    if let Some(meta) = meta {
        client.store_currency_hints(
            symbol,
            CurrencyHints::from_chart(
                meta.currency.as_deref(),
                meta.exchange_name.as_deref(),
                meta.full_exchange_name.as_deref(),
                meta.instrument_type.as_deref(),
            ),
        );
    }
    instrument
}

fn history_adjustment_plan(
    auto_adjust: bool,
    quote: &crate::history::wire::QuoteBlock,
    adjclose: &[WireValue<JsonDecimal>],
    len: usize,
    ctx: &mut ProjectionContext,
) -> Result<Option<AdjustmentPlan>, YfError> {
    if auto_adjust {
        adjustment_plan_for_series(quote, adjclose, len, ctx).map(Some)
    } else {
        Ok(None)
    }
}

async fn history_trading_currency(
    client: &YfClient,
    symbol: &str,
    chart_currency: Option<&str>,
    options: &CallOptions,
    ctx: &mut ProjectionContext,
) -> Result<Option<ResolvedCurrencyUnit>, YfError> {
    let projected = project_currency_resolution(
        ctx,
        symbol,
        CurrencyPurpose::Trading,
        chart_currency,
        client
            .resolve_trading_currency(
                symbol,
                None,
                TradingCurrencyEvidence::ChartMeta(chart_currency),
                options,
            )
            .await,
    )?;

    let issue = projected.issue().cloned();
    if let Some(currency) = projected.into_unit() {
        return Ok(Some(currency));
    }

    ctx.dropped_item(
        "candle",
        None,
        issue.unwrap_or(ProjectionIssue::CurrencyUnresolved),
    )?;
    Ok(None)
}

async fn action_default_currency(
    client: &YfClient,
    symbol: &str,
    events: Option<&crate::history::wire::Events>,
    chart_currency: Option<&str>,
    trading_currency: Option<&ResolvedCurrencyUnit>,
    options: &CallOptions,
    ctx: &mut ProjectionContext,
) -> Result<Option<ResolvedCurrencyUnit>, YfError> {
    if !events_need_default_currency(events) {
        return Ok(trading_currency.cloned());
    }

    if let Some(currency) = trading_currency {
        return Ok(Some(currency.clone()));
    }

    match client
        .resolve_corporate_action_currency(
            symbol,
            None,
            CorporateActionCurrencyEvidence::ChartMeta(chart_currency),
            options,
        )
        .await
    {
        Ok(currency) => {
            ctx.currency_resolution(symbol, CurrencyPurpose::CorporateAction, &currency)?;
            Ok(Some(currency.into_unit()))
        }
        Err(err) => {
            if ctx.policy() == DataQuality::Strict {
                Err(err)
            } else {
                ctx.omitted_present_field(
                    "chart.meta.currency",
                    None,
                    ProjectionIssue::ProviderError {
                        message: err.to_string(),
                    },
                )?;
                Ok(None)
            }
        }
    }
}

fn events_need_default_currency(events: Option<&crate::history::wire::Events>) -> bool {
    let Some(events) = events else {
        return false;
    };

    events.dividends.as_ref().is_some_and(|events| {
        events.values().any(|event| {
            event.amount.as_ref().is_some() && is_missing_currency(event.currency.as_deref())
        })
    }) || events.capital_gains.as_ref().is_some_and(|events| {
        events.values().any(|event| {
            event.amount.as_ref().is_some() && is_missing_currency(event.currency.as_deref())
        })
    })
}

fn is_missing_currency(currency: Option<&str>) -> bool {
    currency.is_none_or(|currency| currency.trim().is_empty())
}

fn map_meta(
    meta: Option<&MetaNode>,
    ctx: &mut ProjectionContext,
) -> Result<Option<HistoryMeta>, YfError> {
    let Some(meta) = meta else {
        return Ok(None);
    };

    let timezone = parse_history_timezone(meta, ctx)?;

    Ok(Some(HistoryMeta {
        timezone,
        utc_offset_seconds: meta.gmtoffset,
    }))
}

fn parse_history_timezone(
    meta: &MetaNode,
    ctx: &mut ProjectionContext,
) -> Result<Option<Tz>, YfError> {
    for (path, field, value) in [
        (
            "chart.meta.exchangeTimezoneName",
            "exchangeTimezoneName",
            meta.exchange_timezone_name.as_deref(),
        ),
        ("chart.meta.timezone", "timezone", meta.timezone.as_deref()),
    ] {
        let Some(timezone) = value.map(str::trim).filter(|timezone| !timezone.is_empty()) else {
            continue;
        };

        match timezone.parse::<Tz>() {
            Ok(timezone) => return Ok(Some(timezone)),
            Err(err) => {
                ctx.omitted_present_field(
                    path,
                    meta.symbol.as_deref(),
                    ProjectionIssue::InvalidField {
                        field,
                        details: err.to_string(),
                    },
                )?;
            }
        }
    }

    Ok(None)
}

fn history_instrument_from_meta(
    requested_symbol: &str,
    meta: Option<&MetaNode>,
) -> Option<Instrument> {
    let meta = meta?;
    let instrument_type = meta
        .instrument_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let kind = parse_yahoo_quote_type(instrument_type).ok()?;

    let exchange = first_parsed_yahoo_exchange([
        meta.full_exchange_name.as_deref(),
        meta.exchange_name.as_deref(),
    ]);

    let instrument = match exchange {
        Some(exchange) => Instrument::from_symbol_and_exchange(requested_symbol, exchange, kind),
        None => Instrument::from_symbol(requested_symbol, kind),
    };

    instrument.ok()
}

fn store_history_instrument(
    client: &YfClient,
    requested_symbol: &str,
    meta: &MetaNode,
    instrument: &Instrument,
) {
    client.store_instrument(requested_symbol.to_string(), instrument.clone());
    if let Some(provider_symbol) = meta
        .symbol
        .as_deref()
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty() && *symbol != requested_symbol)
    {
        client.store_instrument(provider_symbol.to_string(), instrument.clone());
    }
}
