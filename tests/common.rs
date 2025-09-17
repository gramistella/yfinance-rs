#![allow(dead_code)]

use httpmock::{Method::GET, Mock, MockServer};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[must_use]
pub fn setup_server() -> MockServer {
    MockServer::start()
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
pub fn mock_profile_scrape<'a>(server: &'a MockServer, symbol: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/quote/{symbol}"))
            .query_param("p", symbol);
        then.status(200)
            .header("content-type", "text/html")
            .body(fixture("profile_html", symbol, "html"));
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
            .matches(|req| {
                if let Some(group) = &req.query_params {
                    for (k, _) in group {
                        if k == "date" {
                            return false;
                        }
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
    let live = std::env::var("YF_LIVE").ok().as_deref() == Some("1");
    let record = std::env::var("YF_RECORD").ok().as_deref() == Some("1");
    live || record
}

#[must_use]
pub fn is_recording() -> bool {
    std::env::var("YF_RECORD").ok().as_deref() == Some("1")
}
