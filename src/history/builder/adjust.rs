use super::actions::SplitRatio;
use num_bigint::BigUint;
use paft::Decimal;
use std::sync::Arc;

pub type SharedAdjustmentFactor = Arc<AdjustmentFactor>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentError {
    Underflow,
    Overflow,
}

impl AdjustmentError {
    pub const fn diagnostic_target(self) -> &'static str {
        match self {
            Self::Underflow => "adjusted candle prices (Decimal underflow)",
            Self::Overflow => "adjusted candle prices (Decimal overflow)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustmentFactor {
    numerator: BigUint,
    denominator: BigUint,
    exact_close: Option<(Decimal, Decimal)>,
}

impl AdjustmentFactor {
    pub fn new(numerator: Decimal, denominator: Decimal) -> Option<Self> {
        if denominator.is_zero()
            || (!numerator.is_zero()
                && numerator.is_sign_negative() != denominator.is_sign_negative())
        {
            return None;
        }

        let (exact_numerator, exact_denominator) = decimal_ratio(numerator, denominator);
        let mut factor = Self::from_ratio(exact_numerator, exact_denominator)?;
        factor.exact_close = Some((denominator, numerator));
        Some(factor)
    }

    pub(super) fn identity() -> Self {
        Self {
            numerator: BigUint::from(1_u8),
            denominator: BigUint::from(1_u8),
            exact_close: None,
        }
    }

    fn from_split(ratio: SplitRatio) -> Self {
        Self::from_ratio(
            BigUint::from(ratio.denominator().get()),
            BigUint::from(ratio.numerator().get()),
        )
        .expect("split ratios have nonzero components")
    }

    fn from_ratio(mut numerator: BigUint, mut denominator: BigUint) -> Option<Self> {
        if denominator == BigUint::from(0_u8) {
            return None;
        }
        reduce_ratio(&mut numerator, &mut denominator);
        Some(Self {
            numerator,
            denominator,
            exact_close: None,
        })
    }

    fn compose_split(&mut self, ratio: SplitRatio) {
        let factor = Self::from_split(ratio);
        let mut incoming_numerator = factor.numerator;
        let mut incoming_denominator = factor.denominator;
        reduce_ratio(&mut self.numerator, &mut incoming_denominator);
        reduce_ratio(&mut incoming_numerator, &mut self.denominator);
        self.numerator *= incoming_numerator;
        self.denominator *= incoming_denominator;
        self.exact_close = None;
    }

    pub fn apply(&self, value: Decimal) -> Result<Decimal, AdjustmentError> {
        if let Some((close, adjusted_close)) = self.exact_close
            && value == close
        {
            return Ok(adjusted_close);
        }
        if value.is_zero() {
            return Ok(Decimal::ZERO);
        }
        if self.is_identity() {
            return Ok(value);
        }

        let mut value_numerator = BigUint::from(value.mantissa().unsigned_abs());
        let mut factor_numerator = self.numerator.clone();
        let mut decimal_denominator = power_of_ten(value.scale());
        let mut factor_denominator = self.denominator.clone();
        reduce_ratio(&mut value_numerator, &mut decimal_denominator);
        reduce_ratio(&mut value_numerator, &mut factor_denominator);
        reduce_ratio(&mut factor_numerator, &mut decimal_denominator);

        let numerator = value_numerator * factor_numerator;
        let denominator = decimal_denominator * factor_denominator;
        rounded_decimal_ratio(&numerator, &denominator, value.is_sign_negative())
    }

    pub fn is_identity(&self) -> bool {
        self.numerator == self.denominator
    }
}

fn decimal_ratio(numerator: Decimal, denominator: Decimal) -> (BigUint, BigUint) {
    (
        BigUint::from(numerator.mantissa().unsigned_abs()) * power_of_ten(denominator.scale()),
        BigUint::from(denominator.mantissa().unsigned_abs()) * power_of_ten(numerator.scale()),
    )
}

fn power_of_ten(exponent: u32) -> BigUint {
    BigUint::from(10_u8).pow(exponent)
}

fn reduce_ratio(numerator: &mut BigUint, denominator: &mut BigUint) {
    let divisor = greatest_common_divisor(numerator.clone(), denominator.clone());
    *numerator /= &divisor;
    *denominator /= divisor;
}

fn greatest_common_divisor(mut left: BigUint, mut right: BigUint) -> BigUint {
    let zero = BigUint::from(0_u8);
    while right != zero {
        let remainder = &left % &right;
        left = right;
        right = remainder;
    }
    left
}

fn rounded_decimal_ratio(
    numerator: &BigUint,
    denominator: &BigUint,
    negative: bool,
) -> Result<Decimal, AdjustmentError> {
    if numerator == &BigUint::from(0_u8) {
        return Ok(Decimal::ZERO);
    }

    let maximum = BigUint::from(Decimal::MAX.mantissa().unsigned_abs());

    // This is the sole rounding boundary in provider/split adjustment arithmetic.
    // Work downward from Decimal's maximum scale and use round-half-to-even so the
    // returned value retains as much decimal precision as the type can represent.
    for scale in (0..=Decimal::MAX_SCALE).rev() {
        let scaled = numerator * power_of_ten(scale);
        let quotient = &scaled / denominator;
        let remainder = scaled % denominator;
        let twice_remainder = &remainder * 2_u8;
        let odd = quotient.bit(0);
        let mut rounded = quotient;
        let ordering = twice_remainder.cmp(denominator);
        if ordering.is_gt() || (ordering.is_eq() && odd) {
            rounded += 1_u8;
        }

        if rounded > maximum {
            continue;
        }
        if rounded == BigUint::from(0_u8) {
            return Err(AdjustmentError::Underflow);
        }

        let magnitude = u128::try_from(&rounded)
            .ok()
            .and_then(|value| i128::try_from(value).ok())
            .ok_or(AdjustmentError::Overflow)?;
        let mantissa = if negative {
            magnitude.checked_neg().ok_or(AdjustmentError::Overflow)?
        } else {
            magnitude
        };
        return Decimal::try_from_i128_with_scale(mantissa, scale)
            .map(|value| value.normalize())
            .map_err(|_| AdjustmentError::Overflow);
    }

    Err(AdjustmentError::Overflow)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentBasis {
    ProviderAdjusted,
    SplitAdjusted,
}

#[derive(Debug, Clone)]
pub enum AdjustmentPlan {
    ProviderAdjusted {
        row_factors: Vec<Option<AdjustmentFactor>>,
    },
    SplitAdjusted,
}

impl AdjustmentPlan {
    pub const fn basis(&self) -> AdjustmentBasis {
        match self {
            Self::ProviderAdjusted { .. } => AdjustmentBasis::ProviderAdjusted,
            Self::SplitAdjusted => AdjustmentBasis::SplitAdjusted,
        }
    }

    pub fn factor_for_row<'a>(
        &'a self,
        i: usize,
        split_adjustments: &'a [Option<SharedAdjustmentFactor>],
    ) -> Option<&'a AdjustmentFactor> {
        match self {
            Self::ProviderAdjusted { row_factors } => row_factors.get(i).and_then(Option::as_ref),
            Self::SplitAdjusted => split_adjustments.get(i).and_then(Option::as_deref),
        }
    }
}

pub fn split_adjustments_after(
    ts: &[i64],
    split_events: &[(i64, SplitRatio)],
) -> Vec<Option<SharedAdjustmentFactor>> {
    let identity = Arc::new(AdjustmentFactor::identity());
    let mut out = vec![Some(Arc::clone(&identity)); ts.len()];
    if split_events.is_empty() || ts.is_empty() {
        return out;
    }

    let mut sp_idx = split_events.len();
    let mut running = identity;

    for i in (0..ts.len()).rev() {
        if sp_idx > 0 && split_events[sp_idx - 1].0 > ts[i] {
            let mut next = (*running).clone();
            while sp_idx > 0 && split_events[sp_idx - 1].0 > ts[i] {
                sp_idx -= 1;
                next.compose_split(split_events[sp_idx].1);
            }
            running = Arc::new(next);
        }
        out[i] = Some(Arc::clone(&running));
    }
    out
}

pub fn provider_adjustment_factor(
    adjclose_i: Option<Decimal>,
    close_i: Option<Decimal>,
) -> Option<AdjustmentFactor> {
    match (adjclose_i, close_i) {
        (Some(adj), Some(close)) => AdjustmentFactor::new(adj, close),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU32;
    use std::str::FromStr;

    fn decimal(value: &str) -> Decimal {
        Decimal::from_str(value).unwrap()
    }

    #[test]
    fn provider_adjustment_preserves_adjusted_close_exactly() {
        let close = decimal("0.11049100011587143");
        let adjusted_close = decimal("0.08454351872205734");
        let factor = AdjustmentFactor::new(adjusted_close, close).unwrap();

        assert_eq!(factor.apply(close), Ok(adjusted_close));
    }

    #[test]
    fn applying_factor_reduces_exact_ratio_before_multiplication() {
        let factor = AdjustmentFactor::new(
            decimal("0.0276227500289678575"),
            decimal("0.11049100011587143"),
        )
        .unwrap();

        assert_eq!(
            factor.apply(decimal("0.13002200424671173")),
            Ok(decimal("0.0325055010616779325"))
        );
    }

    #[test]
    fn nonterminating_adjustment_rounds_once_at_decimal_capacity() {
        let factor = AdjustmentFactor::new(Decimal::ONE, Decimal::from(3)).unwrap();

        assert_eq!(
            factor.apply(Decimal::ONE),
            Ok(decimal("0.3333333333333333333333333333"))
        );
    }

    #[test]
    fn applying_factor_cross_cancels_value_before_multiplication() {
        let numerator = Decimal::try_from_i128_with_scale(Decimal::MAX.mantissa() / 4, 0)
            .expect("quarter maximum fits");
        let factor = AdjustmentFactor::new(numerator, Decimal::from(4)).unwrap();

        assert_eq!(
            factor.apply(Decimal::from(8)),
            Ok(numerator.checked_mul(Decimal::from(2)).unwrap())
        );
    }

    #[test]
    fn reducing_ratio_cancels_mantissas_before_scale_alignment() {
        let scaled_max = Decimal::try_from_i128_with_scale(Decimal::MAX.mantissa(), 28)
            .expect("scaled maximum fits");
        let value = Decimal::try_from_i128_with_scale(Decimal::MAX.mantissa() / 2, 0)
            .expect("half maximum fits");
        let expected = Decimal::try_from_i128_with_scale(Decimal::MAX.mantissa() / 2, 28)
            .expect("scaled half maximum fits");
        let factor = AdjustmentFactor::new(scaled_max, Decimal::MAX).unwrap();

        assert_eq!(factor.apply(value), Ok(expected));
    }

    #[test]
    fn adjustment_factor_rejects_sign_mismatches() {
        assert!(AdjustmentFactor::new(Decimal::NEGATIVE_ONE, Decimal::ONE).is_none());
        assert!(AdjustmentFactor::new(Decimal::ONE, Decimal::NEGATIVE_ONE).is_none());
        let factor = AdjustmentFactor::new(Decimal::NEGATIVE_ONE, Decimal::from(-2)).unwrap();
        assert_eq!(factor.apply(Decimal::from(-2)), Ok(Decimal::NEGATIVE_ONE));
    }

    #[test]
    fn zero_adjustment_factor_applies_exactly() {
        let factor =
            AdjustmentFactor::new(Decimal::ZERO, decimal("0.0000000000000000000000000001"))
                .unwrap();

        assert_eq!(factor.apply(Decimal::MAX), Ok(Decimal::ZERO));
    }

    #[test]
    fn issue_ten_adjustment_uses_exact_wire_values_and_one_rounding_step() {
        let factor = AdjustmentFactor::new(
            decimal("0.08454351872205734"),
            decimal("0.11049100011587143"),
        )
        .unwrap();

        assert_eq!(
            factor.apply(decimal("0.13002200424671173")),
            Ok(decimal("0.0994879016280374572098676566"))
        );
        assert_eq!(
            factor.apply(decimal("0.13895100355148315")),
            Ok(decimal("0.1063200329247089571168970065"))
        );
        assert_eq!(
            factor.apply(decimal("0.08454351872205734")),
            Ok(decimal("0.0646894910029884437241280919"))
        );
        assert_eq!(
            factor.apply(decimal("0.11049100011587143")),
            Ok(decimal("0.08454351872205734"))
        );
    }

    #[test]
    fn split_adjustments_compose_without_decimal_intermediate_overflow() {
        fn split(numerator: u32, denominator: u32) -> SplitRatio {
            SplitRatio::new(
                NonZeroU32::new(numerator).unwrap(),
                NonZeroU32::new(denominator).unwrap(),
            )
        }

        let max = u32::MAX;
        let mut events = (1..=4)
            .map(|timestamp| (timestamp, split(max, 1)))
            .collect::<Vec<_>>();
        events.extend((5..=7).map(|timestamp| (timestamp, split(1, max))));
        let factor = split_adjustments_after(&[0], &events)
            .pop()
            .flatten()
            .unwrap();

        assert_eq!(factor.apply(Decimal::from(max)), Ok(Decimal::ONE));
    }

    #[test]
    fn split_adjustments_follow_event_order() {
        fn split(numerator: u32, denominator: u32) -> SplitRatio {
            SplitRatio::new(
                NonZeroU32::new(numerator).unwrap(),
                NonZeroU32::new(denominator).unwrap(),
            )
        }

        let factors = split_adjustments_after(&[1, 3, 5], &[(2, split(2, 1)), (4, split(3, 1))]);
        let adjusted = factors
            .into_iter()
            .map(|factor| factor.unwrap().apply(Decimal::from(6)).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(adjusted, [Decimal::ONE, Decimal::from(2), Decimal::from(6)]);
    }

    #[test]
    fn adjustment_rounds_decimal_midpoints_to_even() {
        let factor = AdjustmentFactor::new(Decimal::ONE, Decimal::from(2)).unwrap();
        let quantum = decimal("0.0000000000000000000000000001");

        assert_eq!(factor.apply(quantum), Err(AdjustmentError::Underflow));
        assert_eq!(
            factor.apply(quantum * Decimal::from(3)),
            Ok(quantum * Decimal::from(2))
        );
        assert_eq!(
            factor.apply(quantum * Decimal::from(5)),
            Ok(quantum * Decimal::from(2))
        );
        assert_eq!(
            factor.apply(quantum * Decimal::from(-3)),
            Ok(quantum * Decimal::from(-2))
        );
    }

    #[test]
    fn adjustment_reports_decimal_overflow() {
        let factor = AdjustmentFactor::new(Decimal::from(2), Decimal::ONE).unwrap();

        assert_eq!(factor.apply(Decimal::MAX), Err(AdjustmentError::Overflow));
    }

    #[test]
    fn adjustment_handles_maximum_adjacent_midpoint_carry() {
        let maximum = BigUint::from(Decimal::MAX.mantissa().unsigned_abs());
        let denominator = BigUint::from(2_u8);
        let below = &maximum * 2_u8 - 1_u8;
        let above = &maximum * 2_u8 + 1_u8;
        let expected = Decimal::try_from_i128_with_scale(Decimal::MAX.mantissa() - 1, 0).unwrap();

        assert_eq!(
            rounded_decimal_ratio(&below, &denominator, false),
            Ok(expected)
        );
        assert_eq!(
            rounded_decimal_ratio(&above, &denominator, false),
            Err(AdjustmentError::Overflow)
        );
    }

    #[test]
    fn split_at_bar_timestamp_does_not_adjust_that_bar() {
        let ratio = SplitRatio::new(NonZeroU32::new(2).unwrap(), NonZeroU32::new(1).unwrap());
        let factor = split_adjustments_after(&[2], &[(2, ratio)])
            .pop()
            .flatten()
            .unwrap();

        assert_eq!(factor.apply(Decimal::from(6)), Ok(Decimal::from(6)));
    }

    #[test]
    fn split_accumulator_recovers_after_unrepresentable_intermediate_factor() {
        fn split(numerator: u32, denominator: u32) -> SplitRatio {
            SplitRatio::new(
                NonZeroU32::new(numerator).unwrap(),
                NonZeroU32::new(denominator).unwrap(),
            )
        }

        let max = u32::MAX;
        let mut events = Vec::new();
        for timestamp in 1..=4 {
            events.push((timestamp, split(max, 1)));
        }
        for timestamp in 5..=8 {
            events.push((timestamp, split(1, max)));
        }

        let factors = split_adjustments_after(&[0, 4], &events);
        assert!(
            factors[0]
                .as_ref()
                .is_some_and(|factor| factor.is_identity())
        );
        assert!(
            factors[1]
                .as_ref()
                .is_some_and(|factor| !factor.is_identity())
        );
    }
}
