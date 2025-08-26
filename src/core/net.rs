#[cfg(feature = "test-mode")]
use std::env;

/// Read the response body as text.
/// In `test-mode`, if `YF_RECORD=1`, the body is saved as a fixture via `net_fixtures`.
pub(crate) async fn get_text(
    resp: reqwest::Response,
    _endpoint: &str,
    _symbol: &str,
    _ext: &str,
) -> Result<String, reqwest::Error> {
    let text = resp.text().await?;

    #[cfg(feature = "test-mode")]
    {
        if env::var("YF_RECORD").ok().as_deref() == Some("1")
            && let Err(e) = crate::core::fixtures::record_fixture(_endpoint, _symbol, _ext, &text)
        {
            eprintln!("YF_RECORD: failed to write fixture for {_symbol}: {e}");
        }
    }

    Ok(text)
}
