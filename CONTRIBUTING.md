# Contributing

Thank you for your interest in contributing! This document covers everything you need to get started.

## Prerequisites

| Tool | Install |
|------|---------|
| **Rust** (stable) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **wasm32 target** | `rustup target add wasm32-unknown-unknown` |
| **Trunk** | `cargo install trunk` |

## Development workflow

```sh
# Start the dev server with hot-reload
trunk serve

# Start with verbose logging
RUST_LOG=debug trunk serve

# Run all checks before opening a PR (mirrors CI)
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown -- -D warnings
cargo check --target wasm32-unknown-unknown
```

## Code style

- Formatting is enforced by `rustfmt`. Run `cargo fmt` before committing.
- Linting is enforced by `clippy`. Fix all warnings — CI treats them as errors.

## Submitting changes

1. **Open an issue first** for anything beyond a trivial fix — it avoids duplicated effort.
2. Fork the repository and create a branch from `main`:
   ```sh
   git checkout -b feat/my-feature
   ```
3. Make your changes, keeping commits focused and their messages descriptive.
4. Update `CHANGELOG.md` under the `[Unreleased]` section.
5. Push and open a pull request against `main`.

## Reporting bugs

Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md). Include:

- Steps to reproduce
- Expected vs. actual behaviour
- Rust version (`rustc --version`) and OS

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
