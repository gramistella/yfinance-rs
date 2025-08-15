use std::{fs, path::PathBuf};

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

pub fn write_fixture(name: &str, content: &str) {
    let path = fixtures_dir().join(name);
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content).unwrap();
}

pub fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixtures_dir().join(name)).unwrap()
}
