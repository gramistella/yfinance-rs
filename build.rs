use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=src/stream/yaticker.proto");

    prost_build::compile_protos(&["src/stream/yaticker.proto"], &["src/stream/"]).map_err(|e| {
        // This will ensure the error is printed clearly during the build
        eprintln!("Failed to compile protos: {e}");
        e
    })?;

    Ok(())
}
