//! Test/recording helpers for persisting HTTP fixtures.
//! Compiled only when the `test-mode` feature is enabled.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn get_fixture_dir() -> PathBuf {
    env::var("YF_FIXDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
}

pub(crate) fn record_fixture(
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
