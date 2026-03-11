# Rust Coverage

This repo uses `cargo-llvm-cov` for Rust test coverage.

## Install

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

## Commands

Use the cargo aliases defined in `.cargo/config.toml`:

```bash
cargo cov
cargo cov-html
cargo cov-text
./scripts/coverage
```

Outputs:

- `cargo cov` writes `coverage/lcov.info`
- `cargo cov-html` writes `coverage/html/`
- `cargo cov-text` prints a terminal report
- `./scripts/coverage` creates the coverage directory, builds the HTML report, and opens `coverage/html/html/index.html` with `open` or `xdg-open`
