mod api;
mod model;

/* new: split internals */
mod fetch;
mod wire;

pub use model::{
    BalanceSheetRow, Calendar, CashflowRow, Earnings, EarningsQuarter, EarningsQuarterEps,
    EarningsYear, IncomeStatementRow, Num,
};

use crate::{YfClient, YfError};

pub async fn income_statement(
    client: &mut YfClient,
    symbol: &str,
    quarterly: bool,
) -> Result<Vec<IncomeStatementRow>, YfError> {
    api::income_statement(client, symbol, quarterly).await
}

pub async fn balance_sheet(
    client: &mut YfClient,
    symbol: &str,
    quarterly: bool,
) -> Result<Vec<BalanceSheetRow>, YfError> {
    api::balance_sheet(client, symbol, quarterly).await
}

pub async fn cashflow(
    client: &mut YfClient,
    symbol: &str,
    quarterly: bool,
) -> Result<Vec<CashflowRow>, YfError> {
    api::cashflow(client, symbol, quarterly).await
}

pub async fn earnings(client: &mut YfClient, symbol: &str) -> Result<Earnings, YfError> {
    api::earnings(client, symbol).await
}

pub async fn calendar(client: &mut YfClient, symbol: &str) -> Result<Calendar, YfError> {
    api::calendar(client, symbol).await
}
