# Testing

## Commands

**Offline (replay cached fixtures)**

```bash
cargo test --features test-mode
```

**Full live sweep (no writes; runs all tests including ignored)**

```bash
YF_LIVE=1 cargo test --features test-mode -- --include-ignored --test-threads=1
```

**Record fixtures (live → cache)**

```bash
YF_RECORD=1 cargo test --features test-mode -- --ignored --test-threads=1
```

**Use a different fixture directory (optional)**

```bash
export YF_FIXDIR=/tmp/yf-fixtures
YF_RECORD=1 cargo test --features test-mode -- --ignored --test-threads=1
cargo test --features test-mode
```

**Full test**
YF_RECORD=1 cargo test --features test-mode -- --ignored --test-threads=1 && cargo test --features test-mode