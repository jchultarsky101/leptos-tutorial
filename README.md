# leptos-tutorial

[![CI](https://github.com/jchultarsky101/leptos-tutorial/actions/workflows/ci.yml/badge.svg)](https://github.com/jchultarsky101/leptos-tutorial/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

A hands-on tutorial project for learning [Leptos](https://leptos.dev) — a reactive, full-stack web framework for Rust that compiles to WebAssembly.

## Prerequisites

| Tool | Install |
|------|---------|
| **Rust** (stable) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **wasm32 target** | `rustup target add wasm32-unknown-unknown` |
| **Trunk** (bundler) | `cargo install trunk` |

## Getting started

```sh
git clone https://github.com/jchultarsky101/leptos-tutorial.git
cd leptos-tutorial
trunk serve --open
```

Trunk will compile the project to WebAssembly, bundle it, and open `http://localhost:8080` in your browser. It automatically rebuilds and hot-reloads on file changes.

## Building for production

```sh
trunk build --release
```

The optimised output is written to `dist/`.

## Logging

Tracing is configured via the `RUST_LOG` environment variable **at compile time** (runtime environment variables are unavailable in WebAssembly). Log output goes to the browser's developer console.

```sh
# Serve with debug-level logging
RUST_LOG=debug trunk serve

# Scope logging to the ui crate only
RUST_LOG=ui=trace trunk serve
```

`RUST_LOG` follows the standard [`tracing-subscriber` filter syntax](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html). The default level when `RUST_LOG` is unset is `warn`.

The `api` crate reads `RUST_LOG` at runtime in the normal way:

```sh
RUST_LOG=info cargo run -p api
```

## Project structure

```
├── Cargo.toml          # Cargo workspace root
├── Trunk.toml          # Trunk build configuration
├── common/             # Shared library (used by both ui and api)
│   └── src/lib.rs
├── ui/                 # Leptos frontend — compiles to WebAssembly
│   ├── index.html      # HTML shell
│   └── src/main.rs
└── api/                # Server-side binary — compiles to native
    └── src/main.rs
```

## Contributing

Contributions are welcome! Please open an issue to discuss significant changes before submitting a pull request.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Commit your changes (`git commit -m 'Add my feature'`)
4. Push to the branch (`git push origin feat/my-feature`)
5. Open a pull request

## License

This project is licensed under the [MIT License](LICENSE).
