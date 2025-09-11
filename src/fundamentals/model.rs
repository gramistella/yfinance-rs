// Re-export types from borsa-types
pub use valuta::{
    BalanceSheetRow, Calendar, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps,
    EarningsYear, IncomeStatementRow, ShareCount,
};

/// A common numeric type for financial values, normalized to `f64`.
pub type Num = f64;
