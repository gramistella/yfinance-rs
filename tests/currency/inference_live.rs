use paft::core::domain::Currency;
use yfinance_rs::core::{Interval, Range};
use yfinance_rs::{Ticker, YfClient};

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_reporting_currency_inference() -> Result<(), Box<dyn std::error::Error>> {
    if !crate::common::live_or_record_enabled() {
        return Ok(());
    }

    let client = YfClient::builder().build()?;

    let cases = vec![
        ("AAPL", Currency::USD),
        ("SAP", Currency::EUR),
        ("7203.T", Currency::JPY),
    ];

    for (symbol, expected) in cases {
        let ticker = Ticker::new(&client, symbol);

        let rows = ticker.income_stmt(None).await?;
        let inferred_currency = rows
            .first()
            .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
        assert_eq!(
            inferred_currency,
            Some(expected.clone()),
            "{symbol} expected {expected:?}"
        );

        let cached_before_override = ticker.income_stmt(None).await?;
        let cached_before_currency = cached_before_override
            .first()
            .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
        assert_eq!(
            cached_before_currency,
            Some(expected.clone()),
            "{symbol} cache should retain inferred currency before override"
        );

        let override_rows = ticker.income_stmt(Some(Currency::USD)).await?;
        let override_currency = override_rows
            .first()
            .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
        assert_eq!(
            override_currency,
            Some(Currency::USD),
            "{symbol} override should force USD"
        );

        let second_pass = ticker.income_stmt(None).await?;
        let cached_currency = second_pass
            .first()
            .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
        assert_eq!(
            cached_currency,
            Some(Currency::USD),
            "{symbol} cache should reflect last override"
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "exercise live Yahoo Finance API"]
async fn live_gs2c_dual_listing_currency() -> Result<(), Box<dyn std::error::Error>> {
    if !crate::common::live_or_record_enabled() {
        return Ok(());
    }

    let client = YfClient::builder().build()?;
    let ticker = Ticker::new(&client, "GS2C.DE");

    let fast = ticker.fast_info().await?;
    assert_eq!(fast.currency.as_deref(), Some("EUR"));

    let history = ticker
        .history(Some(Range::D5), Some(Interval::D1), false)
        .await?;
    let history_currency = history.first().map(|bar| bar.close.currency().clone());
    assert_eq!(history_currency, Some(Currency::EUR));

    let fundamentals = ticker.income_stmt(None).await?;
    let fundamentals_currency = fundamentals
        .first()
        .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
    assert_eq!(fundamentals_currency, Some(Currency::USD));

    Ok(())
}
