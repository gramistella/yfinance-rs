#![allow(dead_code)]

use httpmock::{Method::GET, Mock, MockServer};
use std::{fs, path::Path};

pub fn setup_server() -> MockServer {
    MockServer::start()
}

pub fn fixture(endpoint: &str, symbol: &str, ext: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let filename = format!("{}_{}.{}", endpoint, symbol, ext);
    let path = dir.join(&filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
}

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

pub fn mock_history_chart<'a>(server: &'a MockServer, symbol: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v8/finance/chart/{}", symbol));
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("history_chart", symbol, "json"));
    })
}

pub fn mock_profile_api<'a>(server: &'a MockServer, symbol: &'a str, crumb: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{}", symbol))
            .query_param("modules", "assetProfile,quoteType,fundProfile")
            .query_param("crumb", crumb);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("profile_api", symbol, "json"));
    })
}

pub fn mock_profile_scrape<'a>(server: &'a MockServer, symbol: &'a str) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/quote/{}", symbol))
            .query_param("p", symbol);
        then.status(200)
            .header("content-type", "text/html")
            .body(fixture("profile_html", symbol, "html"));
    })
}