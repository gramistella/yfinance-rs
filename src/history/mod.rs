mod builder;
mod wire;

pub use builder::HistoryBuilder;

use crate::core::{
    HistoryRequest, HistoryResponse, HistoryService, YfClient, YfError,
    currency_resolver::ResolvedCurrencyUnit,
};
use paft::domain::Instrument;
use paft::market::responses::history::Candle;
use paft::money::PriceAmount;

const MAX_DECIMAL_SCALE: u32 = 28;

#[derive(Debug)]
pub(crate) struct YahooHistoryResponse {
    pub(crate) response: HistoryResponse,
    pub(crate) price_hint: Option<u32>,
    pub(crate) currency_unit: Option<ResolvedCurrencyUnit>,
    pub(crate) instrument: Option<Instrument>,
}

pub(crate) fn round_candle_prices(
    candles: &mut [Candle],
    price_hint: u32,
    currency_unit: Option<&ResolvedCurrencyUnit>,
) {
    for candle in candles {
        candle.ohlc.open = rounded_price(&candle.ohlc.open, price_hint, currency_unit);
        candle.ohlc.high = rounded_price(&candle.ohlc.high, price_hint, currency_unit);
        candle.ohlc.low = rounded_price(&candle.ohlc.low, price_hint, currency_unit);
        candle.ohlc.close = rounded_price(&candle.ohlc.close, price_hint, currency_unit);
        if let Some(close_unadj) = candle.close_unadj.as_mut() {
            *close_unadj = rounded_price(close_unadj, price_hint, currency_unit);
        }
    }
}

fn rounded_price(
    price: &PriceAmount,
    price_hint: u32,
    currency_unit: Option<&ResolvedCurrencyUnit>,
) -> PriceAmount {
    let price_hint = price_hint.min(MAX_DECIMAL_SCALE);
    if let Some(rounded) = currency_unit
        .and_then(|currency| currency.price_amount_rounded_at_provider_precision(price, price_hint))
    {
        return rounded;
    }

    PriceAmount::new(price.as_decimal().round_dp(price_hint))
}

impl HistoryService for YfClient {
    async fn fetch_full_history(
        &self,
        symbol: &str,
        req: HistoryRequest,
    ) -> Result<HistoryResponse, YfError> {
        // Own everything the async block needs:
        let client = self.clone(); // YfClient: Clone
        let symbol = symbol.to_owned(); // own the symbol
        // HistoryBuilder::new(&YfClient, impl Into<String>) clones internally,
        // so passing &client here is fine.
        let mut hb = builder::HistoryBuilder::new(&client, &symbol)
            .interval(req.interval)
            .auto_adjust(req.auto_adjust)
            .prepost(req.include_prepost)
            .actions(req.include_actions);

        if let Some((p1, p2)) = req.period {
            use chrono::{TimeZone, Utc};
            let start = Utc
                .timestamp_opt(p1, 0)
                .single()
                .ok_or(YfError::InvalidParams("invalid period1".into()))?;
            let end = Utc
                .timestamp_opt(p2, 0)
                .single()
                .ok_or(YfError::InvalidParams("invalid period2".into()))?;
            hb = hb.between(start, end);
        } else if let Some(r) = req.range {
            hb = hb.range(r);
        }

        hb.fetch_full().await
    }
}
