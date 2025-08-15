use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn get_fixture_dir() -> PathBuf {
    env::var("YF_FIXDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
}

fn record_fixture(
    endpoint: &str,
    symbol: &str,
    ext: &str,
    body: &str,
) -> Result<(), std::io::Error> {
    let dir = get_fixture_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let filename = format!("{}_{}.{}", endpoint, symbol, ext);
    let path = dir.join(filename);

    let mut file = fs::File::create(&path)?;
    file.write_all(body.as_bytes())?;

    if env::var("YF_DEBUG").ok().as_deref() == Some("1") {
        eprintln!("YF_RECORD: wrote fixture to {}", path.display());
    }
    Ok(())
}

/// A minimal hook to read response text and optionally record it to a fixture.
pub(crate) async fn get_text(
    resp: reqwest::Response,
    endpoint: &str,
    symbol: &str,
    ext: &str,
) -> Result<String, reqwest::Error> {
    let text = resp.text().await?;

    if env::var("YF_RECORD").ok().as_deref() == Some("1") {
        if let Err(e) = record_fixture(endpoint, symbol, ext, &text) {
            // This is a testing-only feature, so a warning is sufficient.
            eprintln!("YF_RECORD: failed to write fixture for {symbol}: {e}");
        }
    }

    Ok(text)
}