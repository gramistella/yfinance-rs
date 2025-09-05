# Contributing to yfinance-rs

Thanks for considering a contribution! This guide helps you get set up and submit effective pull requests.

## Code of Conduct
Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

### Prerequisites
- Rust (latest stable)
- Cargo
- Git

### Setup
```bash
git clone https://github.com/your-org/yfinance-rs.git
cd yfinance-rs
```

## Development Workflow

### Build
```bash
cargo build
```

### Test
```bash
cargo test
```

### Lint & Format
```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```

## Commit Messages
Use [Conventional Commits](https://www.conventionalcommits.org/) for clear history.

## Pull Requests
1. Create a feature branch.
2. Add tests and documentation as needed.
3. Ensure CI basics pass locally (fmt, clippy, test).
4. Open a PR with a concise description and issue links.

## Release
- Maintainers handle releases following [Semantic Versioning](https://semver.org/).
- Update `CHANGELOG.md` with notable changes.

## Support
Open an issue with details, environment info, and steps to reproduce.
