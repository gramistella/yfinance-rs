use std::fmt::Display;

use yfinance_rs::{
    Decimal, EquityQuery, EtfCategory, EtfQuery, FundCategory, FundQuery, PercentPoints,
    PredefinedScreener, Rating, Region, ResultOffset, ScreenerBuilder, ScreenerCount,
    ScreenerResponse, ScreenerResult, SortDirection, YahooExchangeCode, YfClient, YfError,
    equity_fields, etf_fields, fund_fields, screen,
};

fn display_opt<T: Display>(value: Option<&T>) -> String {
    value.map_or_else(|| "N/A".to_string(), ToString::to_string)
}

#[tokio::main]
async fn main() -> Result<(), YfError> {
    let client = YfClient::default();

    println!("This example makes several live Yahoo Finance screener requests.\n");

    predefined_convenience(&client).await?;
    predefined_builder(&client).await?;
    custom_equity_screen(&client).await?;
    custom_etf_screen(&client).await?;
    custom_fund_screen(&client).await?;

    Ok(())
}

async fn predefined_convenience(client: &YfClient) -> Result<(), YfError> {
    let response = screen(client, PredefinedScreener::DayGainers).await?;
    print_results("Predefined screen via screen(): day gainers", &response);
    Ok(())
}

async fn predefined_builder(client: &YfClient) -> Result<(), YfError> {
    let response = ScreenerBuilder::predefined(client, PredefinedScreener::MostActives)
        .count(ScreenerCount::new(10)?)
        .fetch()
        .await?;

    print_results("Predefined screen via builder: most actives", &response);
    Ok(())
}

async fn custom_equity_screen(client: &YfClient) -> Result<(), YfError> {
    let exchange_filter = equity_fields::EXCHANGE.one_of([
        YahooExchangeCode::Nms,
        YahooExchangeCode::Nyq,
        YahooExchangeCode::Ase,
    ])?;

    let query = EquityQuery::and(vec![
        equity_fields::REGION.eq(Region::Us),
        exchange_filter,
        equity_fields::INTRADAY_PRICE.gte(5),
        equity_fields::INTRADAY_MARKET_CAP.gte(2_000_000_000_u64),
        equity_fields::PERCENT_CHANGE.gt(PercentPoints::new(Decimal::new(20, 1))),
    ])?;

    let response = ScreenerBuilder::equity(client, query)
        .count(ScreenerCount::new(10)?)
        .offset(ResultOffset::new(0))
        .sort_by(equity_fields::PERCENT_CHANGE_SORT, SortDirection::Desc)
        .fetch()
        .await?;

    print_results("Custom equity screen: liquid US gainers", &response);
    Ok(())
}

async fn custom_etf_screen(client: &YfClient) -> Result<(), YfError> {
    let query = EtfQuery::and(vec![
        etf_fields::REGION.eq(Region::Us),
        etf_fields::CATEGORY_NAME.eq(EtfCategory::Technology),
        etf_fields::INTRADAY_PRICE.gt(10),
        etf_fields::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
    ])?;

    let response = ScreenerBuilder::etf(client, query)
        .count(ScreenerCount::new(10)?)
        .sort_by(etf_fields::PERCENT_CHANGE, SortDirection::Desc)
        .fetch()
        .await?;

    print_results("Custom ETF screen: highly rated technology ETFs", &response);
    Ok(())
}

async fn custom_fund_screen(client: &YfClient) -> Result<(), YfError> {
    let query = FundQuery::and(vec![
        fund_fields::CATEGORY_NAME.eq(FundCategory::LargeGrowth),
        fund_fields::PERFORMANCE_RATING_OVERALL.one_of([Rating::Four, Rating::Five])?,
        fund_fields::RISK_RATING_OVERALL.one_of([Rating::One, Rating::Two, Rating::Three])?,
        fund_fields::INITIAL_INVESTMENT.lt(100_001),
        fund_fields::EXCHANGE.eq(YahooExchangeCode::Nas),
    ])?;

    let response = ScreenerBuilder::fund(client, query)
        .count(ScreenerCount::new(10)?)
        .sort_by(fund_fields::FUND_NET_ASSETS, SortDirection::Desc)
        .fetch()
        .await?;

    print_results("Custom fund screen: large-growth funds", &response);
    Ok(())
}

fn print_results(title: &str, response: &ScreenerResponse) {
    println!("--- {title} ---");
    let total = response
        .count
        .map_or_else(|| "unknown".to_string(), |count| count.to_string());
    println!(
        "Yahoo reported {total} total results; showing up to {} rows.",
        response.results.len().min(5)
    );

    for result in response.results.iter().take(5) {
        print_result(result);
    }

    println!();
}

fn print_result(result: &ScreenerResult) {
    let symbol = result.instrument.as_ref().map_or_else(
        || result.symbol.as_deref().unwrap_or("N/A").to_string(),
        ToString::to_string,
    );
    let name = result.name.as_deref().unwrap_or("N/A");
    let exchange = result
        .exchange_display
        .as_deref()
        .or(result.raw_exchange.as_deref())
        .unwrap_or("N/A");
    let quote_type = result.type_display.as_deref().unwrap_or("N/A");
    let price = display_opt(result.price.as_ref());
    let market_cap = display_opt(result.market_cap.as_ref());
    let change = result
        .regular_market_change_percent
        .map_or_else(|| "N/A".to_string(), |value| format!("{value:.2}%"));
    let volume = result
        .regular_market_volume
        .map_or_else(|| "N/A".to_string(), |volume| volume.to_string());

    println!("  {symbol:<14} {name}");
    println!("    type: {quote_type:<12} exchange: {exchange:<12} price: {price}");
    println!("    change: {change:<10} volume: {volume:<14} market cap: {market_cap}");
}
