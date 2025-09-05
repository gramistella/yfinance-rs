// Re-export types from borsa-types
pub use borsa_types::{
    BalanceSheetRow, Calendar, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps,
    EarningsYear, IncomeStatementRow, ShareCount,
};

/// A common numeric type for financial values, normalized to `f64`.
pub type Num = f64;
