use std::env;

/// Read the response body as text.
/// In `test-mode`, if `YF_RECORD=1`, the body is saved as a fixture via `net_fixtures`.
pub(crate) async fn get_text(
    resp: reqwest::Response,
    endpoint: &str,
    symbol: &str,
    ext: &str,
) -> Result<String, reqwest::Error> {
    let text = resp.text().await?;

    #[cfg(feature = "test-mode")]
    {
        if env::var("YF_RECORD").ok().as_deref() == Some("1") {
            if let Err(e) = crate::internal::fixtures::record_fixture(endpoint, symbol, ext, &text) {
                eprintln!("YF_RECORD: failed to write fixture for {symbol}: {e}");
            }
        }
    }

    Ok(text)
}
