# Contributing to yfinance-rs

Thanks for considering a contribution to yfinance-rs! This guide helps you get set up and submit effective pull requests.

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

### Prerequisites

- Rust (latest stable)
- Cargo
- Git
- Just command runner

### Setup

```bash
git clone https://github.com/gramistella/yfinance-rs.git
cd yfinance-rs
```

## Development Workflow

### Test (full test, live recording + offline)

```bash
just test
```

### Offline test

```bash
just test-offline
```

### Lint & Format

```bash
just fmt
just lint
```

To format staged `.rs` and `.toml` files on every commit, install the pre-commit hook (the TOML step needs [`taplo`](https://taplo.tamasfe.dev/)):

```bash
cp hooks/pre-commit .git/hooks/pre-commit
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
