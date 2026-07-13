use super::builder::CustomParts;
use super::fields::{equity_fields as eq, etf_fields as etf, fund_fields as fund};
use super::query::{
    EquityQuery, EquitySector, EtfCategory, EtfQuery, FundCategory, FundQuery, PercentPoints,
    Rating, Region, SortDirection, YahooExchangeCode,
};
use crate::YfError;
use paft::Decimal;

/// Known Yahoo predefined screeners.
///
/// Raw predefined IDs are intentionally not accepted. Add new variants when
/// Yahoo exposes useful new predefined screens.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PredefinedScreener {
    /// Aggressive small-cap equities.
    AggressiveSmallCaps,
    /// Equities with the largest daily gains.
    DayGainers,
    /// Equities with the largest daily losses.
    DayLosers,
    /// Growth-oriented technology stocks.
    GrowthTechnologyStocks,
    /// Most actively traded equities.
    MostActives,
    /// Equities with high short interest.
    MostShortedStocks,
    /// Small-cap gainers.
    SmallCapGainers,
    /// Undervalued growth stocks.
    UndervaluedGrowthStocks,
    /// Undervalued large-cap stocks.
    UndervaluedLargeCaps,
    /// Conservative foreign mutual funds.
    ConservativeForeignFunds,
    /// High-yield bond mutual funds.
    HighYieldBond,
    /// Large-blend portfolio anchor funds.
    PortfolioAnchors,
    /// Solid large-growth funds.
    SolidLargeGrowthFunds,
    /// Solid mid-cap growth funds.
    SolidMidcapGrowthFunds,
    /// Top mutual funds.
    TopMutualFunds,
    /// Top US ETFs.
    TopEtfsUs,
    /// Top-performing ETFs.
    TopPerformingEtfs,
    /// Technology ETFs.
    TechnologyEtfs,
    /// Bond ETFs.
    BondEtfs,
}

impl PredefinedScreener {
    /// Yahoo screener ID sent as `scrIds`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::AggressiveSmallCaps => "aggressive_small_caps",
            Self::DayGainers => "day_gainers",
            Self::DayLosers => "day_losers",
            Self::GrowthTechnologyStocks => "growth_technology_stocks",
            Self::MostActives => "most_actives",
            Self::MostShortedStocks => "most_shorted_stocks",
            Self::SmallCapGainers => "small_cap_gainers",
            Self::UndervaluedGrowthStocks => "undervalued_growth_stocks",
            Self::UndervaluedLargeCaps => "undervalued_large_caps",
            Self::ConservativeForeignFunds => "conservative_foreign_funds",
            Self::HighYieldBond => "high_yield_bond",
            Self::PortfolioAnchors => "portfolio_anchors",
            Self::SolidLargeGrowthFunds => "solid_large_growth_funds",
            Self::SolidMidcapGrowthFunds => "solid_midcap_growth_funds",
            Self::TopMutualFunds => "top_mutual_funds",
            Self::TopEtfsUs => "top_etfs_us",
            Self::TopPerformingEtfs => "top_performing_etfs",
            Self::TechnologyEtfs => "technology_etfs",
            Self::BondEtfs => "bond_etfs",
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn custom_parts(self) -> Result<CustomParts, YfError> {
        let parts = match self {
            Self::AggressiveSmallCaps => CustomParts::equity(
                eq::EOD_VOLUME,
                SortDirection::Desc,
                EquityQuery::and(vec![
                    eq::EXCHANGE.one_of([YahooExchangeCode::Nms, YahooExchangeCode::Nyq])?,
                    eq::EPS_GROWTH_LAST_TWELVE_MONTHS.lt(15),
                ])?,
            ),
            Self::DayGainers => CustomParts::equity(
                eq::PERCENT_CHANGE_SORT,
                SortDirection::Desc,
                EquityQuery::and(vec![
                    eq::PERCENT_CHANGE.gt(PercentPoints::new(Decimal::new(30, 1))),
                    eq::REGION.eq(Region::Us),
                    eq::INTRADAY_MARKET_CAP.gte(2_000_000_000_u64),
                    eq::INTRADAY_PRICE.gte(5),
                    eq::DAY_VOLUME.gt(15_000),
                ])?,
            ),
            Self::DayLosers => CustomParts::equity(
                eq::PERCENT_CHANGE_SORT,
                SortDirection::Asc,
                EquityQuery::and(vec![
                    eq::PERCENT_CHANGE.lt(PercentPoints::new(Decimal::new(-25, 1))),
                    eq::REGION.eq(Region::Us),
                    eq::INTRADAY_MARKET_CAP.gte(2_000_000_000_u64),
                    eq::INTRADAY_PRICE.gte(5),
                    eq::DAY_VOLUME.gt(20_000),
                ])?,
            ),
            Self::GrowthTechnologyStocks => CustomParts::equity(
                eq::EOD_VOLUME,
                SortDirection::Desc,
                EquityQuery::and(vec![
                    eq::QUARTERLY_REVENUE_GROWTH.gte(25),
                    eq::EPS_GROWTH_LAST_TWELVE_MONTHS.gte(25),
                    eq::SECTOR.eq(EquitySector::Technology),
                    eq::EXCHANGE.one_of([YahooExchangeCode::Nms, YahooExchangeCode::Nyq])?,
                ])?,
            ),
            Self::MostActives => CustomParts::equity(
                eq::DAY_VOLUME_SORT,
                SortDirection::Desc,
                EquityQuery::and(vec![
                    eq::REGION.eq(Region::Us),
                    eq::INTRADAY_MARKET_CAP.gte(2_000_000_000_u64),
                    eq::DAY_VOLUME.gt(5_000_000),
                ])?,
            ),
            Self::MostShortedStocks => CustomParts::equity(
                eq::SHORT_PERCENTAGE_OF_SHARES_OUTSTANDING,
                SortDirection::Desc,
                EquityQuery::and(vec![
                    eq::REGION.eq(Region::Us),
                    eq::INTRADAY_PRICE.gt(1),
                    eq::AVERAGE_DAILY_VOLUME_3M.gt(200_000),
                ])?,
            ),
            Self::SmallCapGainers => CustomParts::equity(
                eq::EOD_VOLUME,
                SortDirection::Desc,
                EquityQuery::and(vec![
                    eq::INTRADAY_MARKET_CAP.lt(2_000_000_000_u64),
                    eq::EXCHANGE.one_of([YahooExchangeCode::Nms, YahooExchangeCode::Nyq])?,
                ])?,
            ),
            Self::UndervaluedGrowthStocks => CustomParts::equity(
                eq::EOD_VOLUME,
                SortDirection::Desc,
                EquityQuery::and(vec![
                    eq::PE_RATIO_LAST_TWELVE_MONTHS.between(0, 20),
                    eq::PEG_RATIO_5Y.lt(1),
                    eq::EPS_GROWTH_LAST_TWELVE_MONTHS.gte(25),
                    eq::EXCHANGE.one_of([YahooExchangeCode::Nms, YahooExchangeCode::Nyq])?,
                ])?,
            ),
            Self::UndervaluedLargeCaps => CustomParts::equity(
                eq::EOD_VOLUME,
                SortDirection::Desc,
                EquityQuery::and(vec![
                    eq::PE_RATIO_LAST_TWELVE_MONTHS.between(0, 20),
                    eq::PEG_RATIO_5Y.lt(1),
                    eq::INTRADAY_MARKET_CAP.between(10_000_000_000_u64, 100_000_000_000_u64),
                    eq::EXCHANGE.one_of([YahooExchangeCode::Nms, YahooExchangeCode::Nyq])?,
                ])?,
            ),
            Self::ConservativeForeignFunds => CustomParts::fund(
                fund::FUND_NET_ASSETS,
                SortDirection::Desc,
                FundQuery::and(vec![
                    fund::CATEGORY_NAME.one_of([
                        FundCategory::ForeignLargeValue,
                        FundCategory::ForeignLargeBlend,
                        FundCategory::ForeignLargeGrowth,
                        FundCategory::ForeignSmallMidGrowth,
                        FundCategory::ForeignSmallMidBlend,
                        FundCategory::ForeignSmallMidValue,
                    ])?,
                    fund::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
                    fund::INITIAL_INVESTMENT.lt(100_001),
                    fund::ANNUAL_RETURN_NAV_Y1_CATEGORY_RANK.lt(50),
                    fund::RISK_RATING_OVERALL.one_of([Rating::One, Rating::Two, Rating::Three])?,
                    fund::EXCHANGE.eq(YahooExchangeCode::Nas),
                ])?,
            ),
            Self::HighYieldBond => CustomParts::fund(
                fund::FUND_NET_ASSETS,
                SortDirection::Desc,
                FundQuery::and(vec![
                    fund::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
                    fund::INITIAL_INVESTMENT.lt(100_001),
                    fund::ANNUAL_RETURN_NAV_Y1_CATEGORY_RANK.lt(50),
                    fund::RISK_RATING_OVERALL.one_of([Rating::One, Rating::Two, Rating::Three])?,
                    fund::CATEGORY_NAME.eq(FundCategory::HighYieldBond),
                    fund::EXCHANGE.eq(YahooExchangeCode::Nas),
                ])?,
            ),
            Self::PortfolioAnchors => CustomParts::fund(
                fund::FUND_NET_ASSETS,
                SortDirection::Desc,
                FundQuery::and(vec![
                    fund::CATEGORY_NAME.eq(FundCategory::LargeBlend),
                    fund::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
                    fund::INITIAL_INVESTMENT.lt(100_001),
                    fund::ANNUAL_RETURN_NAV_Y1_CATEGORY_RANK.lt(50),
                    fund::EXCHANGE.eq(YahooExchangeCode::Nas),
                ])?,
            ),
            Self::SolidLargeGrowthFunds => CustomParts::fund(
                fund::FUND_NET_ASSETS,
                SortDirection::Desc,
                FundQuery::and(vec![
                    fund::CATEGORY_NAME.eq(FundCategory::LargeGrowth),
                    fund::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
                    fund::INITIAL_INVESTMENT.lt(100_001),
                    fund::ANNUAL_RETURN_NAV_Y1_CATEGORY_RANK.lt(50),
                    fund::EXCHANGE.eq(YahooExchangeCode::Nas),
                ])?,
            ),
            Self::SolidMidcapGrowthFunds => CustomParts::fund(
                fund::FUND_NET_ASSETS,
                SortDirection::Desc,
                FundQuery::and(vec![
                    fund::CATEGORY_NAME.eq(FundCategory::MidCapGrowth),
                    fund::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
                    fund::INITIAL_INVESTMENT.lt(100_001),
                    fund::ANNUAL_RETURN_NAV_Y1_CATEGORY_RANK.lt(50),
                    fund::EXCHANGE.eq(YahooExchangeCode::Nas),
                ])?,
            ),
            Self::TopMutualFunds => CustomParts::fund(
                fund::PERCENT_CHANGE,
                SortDirection::Desc,
                FundQuery::and(vec![
                    fund::INTRADAY_PRICE.gt(15),
                    fund::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
                    fund::INITIAL_INVESTMENT.gt(1_000),
                    fund::EXCHANGE.eq(YahooExchangeCode::Nas),
                ])?,
            ),
            Self::TopEtfsUs => CustomParts::etf(
                etf::PERCENT_CHANGE,
                SortDirection::Desc,
                EtfQuery::and(vec![
                    etf::INTRADAY_PRICE.gt(10),
                    etf::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
                    etf::REGION.eq(Region::Us),
                ])?,
            ),
            Self::TopPerformingEtfs => CustomParts::etf(
                etf::ANNUAL_REPORT_NET_EXPENSE_RATIO,
                SortDirection::Asc,
                EtfQuery::and(vec![
                    etf::REGION.eq(Region::Us),
                    etf::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
                    etf::INTRADAY_PRICE.gt(10),
                ])?,
            ),
            Self::TechnologyEtfs => CustomParts::etf(
                etf::ANNUAL_REPORT_NET_EXPENSE_RATIO,
                SortDirection::Asc,
                EtfQuery::and(vec![
                    etf::REGION.eq(Region::Us),
                    etf::CATEGORY_NAME.eq(EtfCategory::Technology),
                ])?,
            ),
            Self::BondEtfs => CustomParts::etf(
                etf::ANNUAL_REPORT_NET_EXPENSE_RATIO,
                SortDirection::Asc,
                EtfQuery::and(vec![
                    etf::REGION.eq(Region::Us),
                    etf::CATEGORY_NAME.one_of([
                        EtfCategory::CorporateBond,
                        EtfCategory::EmergingMarketsBond,
                        EtfCategory::EmergingMarketsLocalCurrencyBond,
                        EtfCategory::HighYieldBond,
                        EtfCategory::IntermediateTermBond,
                        EtfCategory::LongTermBond,
                        EtfCategory::InflationProtectedBond,
                        EtfCategory::MultisectorBond,
                        EtfCategory::NontraditionalBond,
                        EtfCategory::ShortTermBond,
                        EtfCategory::UltrashortBond,
                        EtfCategory::WorldBond,
                    ])?,
                ])?,
            ),
        };

        Ok(parts)
    }
}
