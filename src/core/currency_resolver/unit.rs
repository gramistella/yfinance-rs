use crate::core::YfError;
use paft::money::{Currency, Money, Price, PriceAmount};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedCurrencyUnit {
    currency: Currency,
    scale: PriceScale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PriceScale {
    Major,
    Hundredth,
}

impl ResolvedCurrencyUnit {
    pub const fn from_currency(currency: Currency) -> Self {
        Self {
            currency,
            scale: PriceScale::Major,
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            return None;
        }

        let (code, scale) = match trimmed {
            "GBp" | "GBX" => ("GBP", PriceScale::Hundredth),
            "ZAc" => ("ZAR", PriceScale::Hundredth),
            "ILA" => ("ILS", PriceScale::Hundredth),
            _ => (trimmed, PriceScale::Major),
        };

        Currency::from_str(code)
            .ok()
            .map(|currency| Self { currency, scale })
    }

    pub fn major_from_code(code: &str) -> Option<Self> {
        Self::from_code(code).map(|unit| unit.major_unit())
    }

    pub fn major_unit(&self) -> Self {
        Self::from_currency(self.currency.clone())
    }

    pub const fn currency(&self) -> &Currency {
        &self.currency
    }

    pub(crate) fn price_amount_rounded_at_provider_precision(
        &self,
        value: &PriceAmount,
        precision: u32,
    ) -> Option<PriceAmount> {
        let provider_value = self.provider_units_from_major(*value.as_decimal())?;
        self.scaled_decimal(provider_value.round_dp(precision))
            .map(PriceAmount::new)
    }

    pub fn price_amount_from_decimal(&self, value: Decimal) -> Option<PriceAmount> {
        self.scaled_decimal(value).map(PriceAmount::new)
    }

    pub fn price_from_decimal(&self, value: Decimal) -> Option<Price> {
        self.scaled_decimal(value)
            .map(|decimal| Price::new(decimal, self.currency.clone()))
    }

    pub fn money_from_i64(&self, value: i64) -> Result<Money, YfError> {
        let decimal = Decimal::from_i128_with_scale(i128::from(value), 0);
        self.money_from_decimal(decimal)
    }

    pub fn money_from_u64(&self, value: u64) -> Result<Money, YfError> {
        let decimal = Decimal::from_i128_with_scale(i128::from(value), 0);
        self.money_from_decimal(decimal)
    }

    pub fn money_from_decimal(&self, decimal: Decimal) -> Result<Money, YfError> {
        Ok(Money::new(decimal, self.currency.clone())?)
    }

    fn scaled_decimal(&self, value: Decimal) -> Option<Decimal> {
        match self.scale {
            PriceScale::Major => Some(value),
            PriceScale::Hundredth => checked_decimal_product(value, Decimal::new(1, 2)),
        }
    }

    fn provider_units_from_major(&self, value: Decimal) -> Option<Decimal> {
        match self.scale {
            PriceScale::Major => Some(value),
            PriceScale::Hundredth => checked_decimal_product(value, Decimal::from(100)),
        }
    }
}

fn checked_decimal_product(left: Decimal, right: Decimal) -> Option<Decimal> {
    let mut mantissa = left.mantissa().checked_mul(right.mantissa())?;
    let mut scale = left.scale().checked_add(right.scale())?;

    loop {
        if let Ok(value) = Decimal::try_from_i128_with_scale(mantissa, scale) {
            return Some(value);
        }
        if scale == 0 || mantissa % 10 != 0 {
            return None;
        }
        mantissa /= 10;
        scale -= 1;
    }
}
