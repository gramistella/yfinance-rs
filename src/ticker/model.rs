#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub symbol: String,
    pub regular_market_price: Option<f64>,
    pub regular_market_previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FastInfo {
    pub symbol: String,
    pub last_price: f64,
    pub previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionContract {
    pub contract_symbol: String,
    pub strike: f64,
    pub last_price: Option<f64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub volume: Option<u64>,
    pub open_interest: Option<u64>,
    pub implied_volatility: Option<f64>,
    pub in_the_money: bool,
    pub expiration: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionChain {
    pub calls: Vec<OptionContract>,
    pub puts: Vec<OptionContract>,
}
