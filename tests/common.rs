#![allow(dead_code)]

use httpmock::{HttpMockRequest, Method::GET, Mock, MockServer};
#[cfg(feature = "tracing-subscriber")]
use std::sync::OnceLock;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const KEY_STATISTICS_MODULES: &str = "summaryDetail,defaultKeyStatistics";
pub const KEY_STATISTICS_FIXTURE_ENDPOINT: &str =
    "key_statistics_api_summaryDetail-defaultKeyStatistics";
const DAILY_MAX_PERIOD_SPAN_SECONDS: i64 = 3_122_063_995;
const SECONDS_PER_DAY: i64 = 86_400;

#[cfg(feature = "tracing-subscriber")]
static TEST_TRACING: OnceLock<()> = OnceLock::new();

pub fn init_tracing() {
    #[cfg(feature = "tracing-subscriber")]
    {
        TEST_TRACING.get_or_init(yfinance_rs::init_tracing_for_tests);
    }
}

#[must_use]
pub fn setup_server() -> MockServer {
    init_tracing();
    MockServer::start()
}

#[must_use]
pub fn is_daily_max_period_query(request: &HttpMockRequest) -> bool {
    let params = request.query_params_map();
    let period = params
        .get("period1")
        .and_then(|value| value.parse::<i64>().ok())
        .zip(
            params
                .get("period2")
                .and_then(|value| value.parse::<i64>().ok()),
        );

    !params.contains_key("range")
        && params.get("interval").is_some_and(|value| value == "1d")
        && period.is_some_and(|(start, end)| {
            end.checked_sub(start) == Some(DAILY_MAX_PERIOD_SPAN_SECONDS)
                && end.rem_euclid(SECONDS_PER_DAY) == 0
                && end
                    .checked_sub(chrono::Utc::now().timestamp())
                    .is_some_and(|lead| (-60..=SECONDS_PER_DAY).contains(&lead))
        })
}

fn fixture_dir() -> PathBuf {
    std::env::var("YF_FIXDIR").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        PathBuf::from,
    )
}

#[must_use]
/// Returns fixture file contents for a given endpoint/symbol/extension.
///
/// # Panics
///
/// Panics if the fixture file cannot be read.
pub fn fixture(endpoint: &str, symbol: &str, ext: &str) -> String {
    init_tracing();
    let path = fixture_path(endpoint, symbol, ext);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
}

#[must_use]
pub fn fixture_path(endpoint: &str, symbol: &str, ext: &str) -> PathBuf {
    fixture_dir().join(format!("{endpoint}_{symbol}.{ext}"))
}

#[must_use]
pub fn fixture_exists(endpoint: &str, symbol: &str, ext: &str) -> bool {
    fixture_path(endpoint, symbol, ext).exists()
}

#[must_use]
/// Extracts beta from a recorded quoteSummary key statistics fixture.
///
/// # Panics
///
/// Panics if the fixture is not valid JSON or does not contain beta in
/// `summaryDetail` or `defaultKeyStatistics`.
pub fn quote_summary_beta(fixture: &str) -> paft::Decimal {
    let raw: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let result = raw["quoteSummary"]["result"]
        .as_array()
        .and_then(|results| results.first())
        .expect("quoteSummary fixture should contain a result");
    let beta = result["summaryDetail"]["beta"]["raw"]
        .as_f64()
        .or_else(|| result["defaultKeyStatistics"]["beta"]["raw"].as_f64())
        .expect("quoteSummary fixture should contain beta");

    paft::Decimal::try_from(beta).unwrap()
}

#[must_use]
/// Extracts `summaryDetail.exDividendDate` from a recorded quoteSummary key statistics fixture.
///
/// # Panics
///
/// Panics if the fixture is not valid JSON or does not contain an ex-dividend date.
pub fn quote_summary_ex_dividend_date(fixture: &str) -> chrono::NaiveDate {
    let raw: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let result = raw["quoteSummary"]["result"]
        .as_array()
        .and_then(|results| results.first())
        .expect("quoteSummary fixture should contain a result");
    let timestamp = result["summaryDetail"]["exDividendDate"]["raw"]
        .as_i64()
        .expect("quoteSummary fixture should contain summaryDetail.exDividendDate");

    chrono::DateTime::from_timestamp(timestamp, 0)
        .expect("exDividendDate should be in range")
        .date_naive()
}

#[must_use]
pub fn mock_cookie_crumb(server: &'_ MockServer) -> (Mock<'_>, Mock<'_>) {
    let cookie_mock = server.mock(|when, then| {
        when.method(GET).path("/consent");
        then.status(200).header(
            "set-cookie",
            "A=B; Max-Age=315360000; Domain=.yahoo.com; Path=/; Secure; SameSite=None",
        );
    });
    let crumb_mock = server.mock(|when, then| {
        when.method(GET).path("/v1/test/getcrumb");
        then.status(200).body("crumb-value");
    });
    (cookie_mock, crumb_mock)
}

#[must_use]
pub fn mock_history_chart<'a>(server: &'a MockServer, symbol: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET).path(format!("/v8/finance/chart/{symbol}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("history_chart", symbol, "json"));
    })
}

#[must_use]
pub fn mock_profile_api<'a>(server: &'a MockServer, symbol: &'a str, crumb: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{symbol}"))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("profile_api", symbol, "json"));
    })
}

#[must_use]
pub fn mock_quote_v7<'a>(server: &'a MockServer, symbol: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", symbol);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("quote_v7", symbol, "json"));
    })
}

#[must_use]
pub fn mock_quote_v7_multi<'a>(server: &'a MockServer, symbols_csv: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path("/v7/finance/quote")
            .query_param("symbols", symbols_csv);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("quote_v7", "MULTI", "json"));
    })
}

#[must_use]
pub fn mock_options_v7<'a>(server: &'a MockServer, symbol: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v7/finance/options/{symbol}"))
            .is_true(|req| {
                let group = req.query_params();
                for (k, _) in group {
                    if k == "date" {
                        return false;
                    }
                }
                true
            });
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("options_v7", symbol, "json"));
    })
}

#[must_use]
pub fn mock_options_v7_for_date<'a>(
    server: &'a MockServer,
    symbol: &'a str,
    date: i64,
) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v7/finance/options/{symbol}"))
            .query_param("date", date.to_string());
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("options_v7", &format!("{symbol}_{date}"), "json"));
    })
}

#[must_use]
pub fn live_or_record_enabled() -> bool {
    init_tracing();
    let live = std::env::var("YF_LIVE").ok().as_deref() == Some("1");
    let record = std::env::var("YF_RECORD").ok().as_deref() == Some("1");
    live || record
}

#[must_use]
pub fn is_recording() -> bool {
    init_tracing();
    std::env::var("YF_RECORD").ok().as_deref() == Some("1")
}
