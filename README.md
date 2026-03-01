# leptos-tutorial

[![CI](https://github.com/jchultarsky101/leptos-tutorial/actions/workflows/ci.yml/badge.svg)](https://github.com/jchultarsky101/leptos-tutorial/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

A hands-on tutorial project for learning [Leptos](https://leptos.dev) — a reactive, full-stack web framework for Rust that compiles to WebAssembly. The project also includes a REST API backend (Axum) that implements a virtual file catalog with folder hierarchies.

## Prerequisites

| Tool | Install |
|------|---------|
| **Rust** (stable) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **wasm32 target** | `rustup target add wasm32-unknown-unknown` |
| **Trunk** (WASM bundler) | `cargo install trunk` |

## Running locally

The project has two independent processes. Start each in its own terminal.

### 1 — API server (backend)

```sh
cargo run -p api
```

### 2 — UI dev server (frontend)

```sh
trunk serve
```

Trunk compiles the project to WebAssembly, serves it, and automatically rebuilds and hot-reloads on every file change.

## Default URLs

| Service | URL | Notes |
|---------|-----|-------|
| **UI** | <http://localhost:8080> | Leptos CSR frontend served by Trunk |
| **API** | <http://localhost:3000> | Axum REST server |
| **Swagger UI** | <http://localhost:3000/swagger-ui/> | Interactive API docs with try-it-out |
| **OpenAPI JSON** | <http://localhost:3000/api-doc/openapi.json> | Raw OpenAPI 3 spec |

## Environment variables

### API server

| Variable | Default | Description |
|----------|---------|-------------|
| `CATALOG_PORT` | `3000` | TCP port the API listens on |
| `CATALOG_STORAGE_DIR` | `./data` | Directory used to store uploaded file content |
| `RUST_LOG` | _(unset — silent)_ | Log filter, e.g. `info`, `debug`, `api=trace` |

```sh
# Run on a non-default port with verbose logging
CATALOG_PORT=4000 RUST_LOG=debug cargo run -p api
```

### UI dev server (Trunk)

| Variable / flag | Default | Description |
|-----------------|---------|-------------|
| `RUST_LOG` | `warn` | Captured **at compile time** — restart Trunk to apply changes |
| `--port` | `8080` | Trunk's HTTP port |
| `--open` | _(off)_ | Open the browser automatically on start |

```sh
# Run on port 3001 and open the browser
trunk serve --port 3001 --open

# Build with debug logging compiled into the WASM binary
RUST_LOG=debug trunk serve
```

> **Note:** WebAssembly cannot read environment variables at runtime. `RUST_LOG` is captured by Trunk **at compile time**, so you must restart `trunk serve` after changing it.

## Testing the API with curl

The **Swagger UI** at <http://localhost:3000/swagger-ui/> provides a browser-based try-it-out interface for every endpoint. For command-line testing, use the examples below (requires a running API server).

> Install [jq](https://jqlang.github.io/jq/) to pretty-print responses (`brew install jq` on macOS).

### Folders

```sh
# List root
curl -s http://localhost:3000/folders | jq

# Create /docs under root
curl -s -X POST http://localhost:3000/folders \
  -H 'Content-Type: application/json' \
  -d '{"name":"docs"}' | jq

# Create /docs/reports (nested)
curl -s -X POST http://localhost:3000/folders/docs \
  -H 'Content-Type: application/json' \
  -d '{"name":"reports"}' | jq

# List /docs
curl -s http://localhost:3000/folders/docs | jq

# Rename /docs → /documentation
curl -s -X PATCH http://localhost:3000/folder-rename/docs \
  -H 'Content-Type: application/json' \
  -d '{"new_name":"documentation"}' | jq

# Move /documentation/reports into /archive
#   (create /archive first)
curl -s -X POST http://localhost:3000/folders \
  -H 'Content-Type: application/json' \
  -d '{"name":"archive"}' | jq

curl -s -X PATCH http://localhost:3000/folder-move/documentation/reports \
  -H 'Content-Type: application/json' \
  -d '{"new_parent_path":"/archive"}' | jq

# Delete /documentation and all its contents
curl -s -X DELETE http://localhost:3000/folders/documentation
```

### Files

```sh
# Upload README.md as /docs/readme.txt
#   (create /docs first if it doesn't exist)
curl -s -X POST http://localhost:3000/files/docs/readme.txt \
  -F 'file=@./README.md' | jq

# Download it
curl -s http://localhost:3000/files/docs/readme.txt -o downloaded.txt

# Replace its content (re-upload)
curl -s -X PUT http://localhost:3000/files/docs/readme.txt \
  -F 'file=@./CHANGELOG.md' | jq

# Rename /docs/readme.txt → /docs/notes.txt
curl -s -X PATCH http://localhost:3000/file-rename/docs/readme.txt \
  -H 'Content-Type: application/json' \
  -d '{"new_name":"notes.txt"}' | jq

# Move /docs/notes.txt into /archive
curl -s -X PATCH http://localhost:3000/file-move/docs/notes.txt \
  -H 'Content-Type: application/json' \
  -d '{"new_folder_path":"/archive"}' | jq

# Delete a file
curl -s -X DELETE http://localhost:3000/files/archive/notes.txt
```

## Running the tests

```sh
# All unit tests (common + api, 28 tests)
cargo test -p common -p api

# With log output visible
cargo test -p common -p api -- --nocapture

# One specific test by name
cargo test -p api rename_folder_updates_paths
```

## Building for production

```sh
# API — native release binary (output: target/release/api)
cargo build --release -p api

# UI — optimised WASM bundle (output: dist/)
trunk build --release
```

## Logging

The `api` crate reads `RUST_LOG` at runtime:

```sh
RUST_LOG=info cargo run -p api
RUST_LOG=api=debug,tower_http=trace cargo run -p api
```

The `ui` crate reads `RUST_LOG` **at compile time** (WebAssembly has no access to environment variables at runtime). Log output appears in the browser developer console.

```sh
RUST_LOG=debug trunk serve         # debug for all modules
RUST_LOG=ui=trace trunk serve      # trace for the ui crate only
```

`RUST_LOG` follows the standard [`tracing-subscriber` filter syntax](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html). The default level when unset is `warn`.

## Project structure

```
├── Cargo.toml              # Cargo workspace root
├── Trunk.toml              # Trunk build config (entry: ui/index.html)
├── common/                 # Shared library — compiled for both WASM and native
│   └── src/
│       ├── path.rs         # CatalogPath newtype with validation
│       ├── error.rs        # CatalogError (thiserror)
│       ├── model.rs        # Domain models (FolderEntry, FileEntry)
│       └── dto.rs          # REST DTOs with serde / utoipa / garde derives
├── ui/                     # Leptos frontend — compiles to WebAssembly
│   ├── index.html          # HTML shell loaded by Trunk
│   └── src/main.rs
└── api/                    # Axum REST server — compiles to native
    └── src/
        ├── main.rs         # Entry point, env config, server startup
        ├── state.rs        # AppState (Arc<dyn MetadataStore + FileStore>)
        ├── error.rs        # ApiError → HTTP status mapping
        ├── routes/
        │   ├── mod.rs      # Router assembly + OpenAPI doc
        │   ├── folders.rs  # Folder CRUD handlers
        │   └── files.rs    # File upload / download / management handlers
        └── storage/
            ├── mod.rs      # MetadataStore + FileStore traits
            ├── memory.rs   # In-memory metadata store (default)
            └── filesystem.rs # Local filesystem file store
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
