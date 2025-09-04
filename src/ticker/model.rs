use serde::Serialize;

#[cfg(feature = "dataframe")]
use borsa_macros::ToDataFrame;

#[cfg(feature = "dataframe")]
use crate::core::dataframe::ToDataFrame;

/// A compact quote summary, useful for quick price checks.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct FastInfo {
    /// The ticker symbol.
    pub symbol: String,
    /// The last traded price, falling back to the previous close if not available.
    pub last_price: f64,
    /// The closing price of the previous regular market session.
    pub previous_close: Option<f64>,
    /// The currency in which the security is traded.
    pub currency: Option<String>,
    /// The full name of the exchange where the security is traded.
    pub exchange: Option<String>,
    /// The current state of the market for this security.
    pub market_state: Option<String>,
}

/// Represents a single options contract (a call or a put).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "dataframe", derive(ToDataFrame))]
pub struct OptionContract {
    /// The unique symbol for the options contract.
    pub contract_symbol: String,
    /// The strike price of the contract.
    pub strike: f64,
    /// The last traded price for this contract.
    pub last_price: Option<f64>,
    /// The current bid price for the contract.
    pub bid: Option<f64>,
    /// The current ask price for the contract.
    pub ask: Option<f64>,
    /// The trading volume for the current day.
    pub volume: Option<u64>,
    /// The number of contracts held by traders that have not been closed or exercised.
    pub open_interest: Option<u64>,
    /// The implied volatility of the contract.
    pub implied_volatility: Option<f64>,
    /// Whether the contract is currently in the money.
    pub in_the_money: bool,
    /// The Unix timestamp (in seconds) of the contract's expiration date.
    pub expiration: i64,
}

/// A full options chain for a specific expiration date, containing both calls and puts.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionChain {
    /// A list of all call contracts for the expiration date.
    pub calls: Vec<OptionContract>,
    /// A list of all put contracts for the expiration date.
    pub puts: Vec<OptionContract>,
}
