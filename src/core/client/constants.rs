//! Centralized constants for default endpoints and UA.

/// Default desktop UA to avoid trivial bot blocking.
pub(crate) const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (X11; Linux x86_64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/122.0.0.0 Safari/537.36"
);

/// Yahoo chart API base (symbol is appended).
pub(crate) const DEFAULT_BASE_CHART: &str = "https://query1.finance.yahoo.com/v8/finance/chart/";

/// Yahoo quote HTML base (symbol is appended).
pub(crate) const DEFAULT_BASE_QUOTE: &str = "https://finance.yahoo.com/quote/";

/// Yahoo quoteSummary API base (symbol is appended).
pub(crate) const DEFAULT_BASE_QUOTE_API: &str =
    "https://query1.finance.yahoo.com/v10/finance/quoteSummary/";

/// A URL that returns a Set-Cookie header for Yahoo domains.
pub(crate) const DEFAULT_COOKIE_URL: &str = "https://fc.yahoo.com/consent";

/// URL to fetch a crumb (requires cookie from `DEFAULT_COOKIE_URL`).
pub(crate) const DEFAULT_CRUMB_URL: &str = "https://query1.finance.yahoo.com/v1/test/getcrumb";

/// Base URL for the Yahoo Finance v7 quote API.
pub(crate) const DEFAULT_BASE_QUOTE_V7: &str = "https://query1.finance.yahoo.com/v7/finance/quote";

/// Base URL for the Yahoo Finance v7 options API.
pub(crate) const DEFAULT_BASE_OPTIONS_V7: &str = "https://query1.finance.yahoo.com/v7/finance/options/";

/// Base URL for the Yahoo Finance search API.
pub(crate) const DEFAULT_BASE_STREAM: &str = "wss://streamer.finance.yahoo.com/?version=2";


