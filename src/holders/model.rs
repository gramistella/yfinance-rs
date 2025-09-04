// src/holders/model.rs

use serde::Serialize;

#[cfg(feature = "dataframe")]
use borsa_macros::ToDataFrame;

#[cfg(feature = "dataframe")]
use crate::core::dataframe::ToDataFrame;


/// Represents a single row in the major holders breakdown table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct MajorHolder {
    /// The category of the holder (e.g., "% of Shares Held by All Insider").
    pub category: String,
    /// The value associated with the category, usually a percentage formatted as a string.
    pub value: String,
}

/// Represents a single institutional or mutual fund holder.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct InstitutionalHolder {
    /// The name of the holding institution or fund.
    pub holder: String,
    /// The number of shares held.
    pub shares: u64,
    /// The date of the last reported position as a Unix timestamp.
    pub date_reported: i64,
    /// The percentage of the company's outstanding shares held by this entity.
    pub pct_held: f64,
    /// The market value of the shares held.
    pub value: u64,
}

/// Represents a single insider transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct InsiderTransaction {
    /// The name of the insider who executed the transaction.
    pub insider: String,
    /// The insider's relationship to the company (e.g., "Officer").
    pub position: String,
    /// A description of the transaction (e.g., "Sale").
    pub transaction: String,
    /// The number of shares involved in the transaction.
    pub shares: u64,
    /// The total value of the transaction.
    pub value: u64,
    /// The start date of the transaction as a Unix timestamp.
    pub start_date: i64,
    /// A URL to the source filing for the transaction, if available.
    pub url: String,
}

/// Represents a single insider on the company's roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct InsiderRosterHolder {
    /// The name of the insider.
    pub name: String,
    /// The insider's position in the company.
    pub position: String,
    /// A description of the most recent transaction made by this insider.
    pub most_recent_transaction: String,
    /// The date of the latest transaction as a Unix timestamp.
    pub latest_transaction_date: i64,
    /// The number of shares owned directly by the insider.
    pub shares_owned_directly: u64,
    /// The date of the direct ownership filing as a Unix timestamp.
    pub position_direct_date: i64,
}

/// A summary of net share purchase activity by insiders over a specific period.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct NetSharePurchaseActivity {
    /// The period the summary covers (e.g., "3m").
    pub period: String,
    /// The total number of shares purchased by insiders.
    pub buy_info_shares: u64,
    /// The number of separate buy transactions.
    pub buy_info_count: u64,
    /// The total number of shares sold by insiders.
    pub sell_info_shares: u64,
    /// The number of separate sell transactions.
    pub sell_info_count: u64,
    /// The net number of shares purchased or sold.
    pub net_info_shares: i64,
    /// The net number of transactions.
    pub net_info_count: i64,
    /// The total number of shares held by all insiders.
    pub total_insider_shares: u64,
    /// The net shares purchased/sold as a percentage of total insider shares.
    pub net_percent_insider_shares: f64,
}
