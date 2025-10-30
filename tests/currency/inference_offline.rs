use httpmock::Method::GET;
use httpmock::MockServer;
use paft::money::{Currency, IsoCurrency};
use url::Url;
use yfinance_rs::core::{Interval, Range};
use yfinance_rs::{ApiPreference, Ticker, YfClient};

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

fn has_fixture(endpoint: &str, symbol: &str) -> bool {
    crate::common::fixture_exists(endpoint, symbol, "json")
}

#[tokio::test]
async fn offline_currency_inference_uses_profile_country() {
    let symbol = "TSCO.L";

    assert!(
        has_fixture("profile_api_assetProfile-quoteType-fundProfile", symbol),
        "missing fixture profile_api_assetProfile-quoteType-fundProfile_{symbol}.json"
    );
    assert!(
        has_fixture("timeseries_income_statement_annual", symbol),
        "missing fixture timeseries_income_statement_annual_{symbol}.json"
    );

    let server = MockServer::start();

    let profile_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{symbol}"))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture(
                "profile_api_assetProfile-quoteType-fundProfile",
                symbol,
            ));
    });

    let fundamentals_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("timeseries_income_statement_annual", symbol));
    });
    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .base_timeseries(
            Url::parse(&format!(
                "{}/ws/fundamentals-timeseries/v1/finance/timeseries/",
                server.base_url()
            ))
            .unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, symbol);

    let rows = ticker.income_stmt(None).await.unwrap();
    let inferred_currency = rows
        .first()
        .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
    assert_eq!(inferred_currency, Some(Currency::Iso(IsoCurrency::GBP)));

    let cached_before_override = ticker.income_stmt(None).await.unwrap();
    let cached_before_currency = cached_before_override
        .first()
        .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
    assert_eq!(
        cached_before_currency,
        Some(Currency::Iso(IsoCurrency::GBP))
    );

    let rows_override = ticker
        .income_stmt(Some(Currency::Iso(IsoCurrency::USD)))
        .await
        .unwrap();
    let override_currency = rows_override
        .first()
        .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
    assert_eq!(override_currency, Some(Currency::Iso(IsoCurrency::USD)));

    let rows_cached = ticker.income_stmt(None).await.unwrap();
    let cached_currency = rows_cached
        .first()
        .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
    assert_eq!(cached_currency, Some(Currency::Iso(IsoCurrency::USD)));

    assert_eq!(profile_mock.calls(), 1, "profile should be fetched once");
    assert_eq!(
        fundamentals_mock.calls(),
        4,
        "fundamentals should be fetched four times"
    );
}

#[tokio::test]
async fn offline_gs2c_dual_listing_currency() {
    let symbol = "GS2C.DE";

    assert!(
        has_fixture("quote_v7", symbol),
        "missing fixture quote_v7_{symbol}.json"
    );
    assert!(
        has_fixture("profile_api_assetProfile-quoteType-fundProfile", symbol),
        "missing fixture profile_api_assetProfile-quoteType-fundProfile_{symbol}.json"
    );
    assert!(
        has_fixture("timeseries_income_statement_annual", symbol),
        "missing fixture timeseries_income_statement_annual_{symbol}.json"
    );
    assert!(
        has_fixture("history_chart", symbol),
        "missing fixture history_chart_{symbol}.json"
    );

    let server = MockServer::start();

    let quote_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", symbol);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("quote_v7", symbol));
    });

    let profile_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{symbol}"))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture(
                "profile_api_assetProfile-quoteType-fundProfile",
                symbol,
            ));
    });

    let fundamentals_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/ws/fundamentals-timeseries/v1/finance/timeseries/{symbol}"
            ))
            .query_param_exists("type")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("timeseries_income_statement_annual", symbol));
    });

    let chart_mock = server.mock(|when, then| {
        when.method(GET).path(format!("/v8/finance/chart/{symbol}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("history_chart", symbol));
    });

    let client = YfClient::builder()
        .base_quote_v7(Url::parse(&format!("{}/v7/finance/quote", server.base_url())).unwrap())
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        .base_timeseries(
            Url::parse(&format!(
                "{}/ws/fundamentals-timeseries/v1/finance/timeseries/",
                server.base_url()
            ))
            .unwrap(),
        )
        .base_chart(Url::parse(&format!("{}/v8/finance/chart/", server.base_url())).unwrap())
        ._api_preference(ApiPreference::ApiOnly)
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, symbol);

    let fast = ticker.fast_info().await.unwrap();
    assert_eq!(fast.currency.map(|c| c.to_string()).as_deref(), Some("EUR"));

    let fundamentals = ticker.income_stmt(None).await.unwrap();
    let fundamentals_currency = fundamentals
        .first()
        .and_then(|row| row.total_revenue.as_ref().map(|m| m.currency().clone()));
    assert_eq!(fundamentals_currency, Some(Currency::Iso(IsoCurrency::USD)));

    let history = ticker
        .history(Some(Range::D5), Some(Interval::D1), false)
        .await
        .unwrap();
    let history_currency = history.first().map(|bar| bar.close.currency().clone());
    assert_eq!(history_currency, Some(Currency::Iso(IsoCurrency::EUR)));

    assert_eq!(quote_mock.calls(), 1);
    assert_eq!(profile_mock.calls(), 1);
    assert_eq!(fundamentals_mock.calls(), 1);
    assert_eq!(chart_mock.calls(), 1);
}
