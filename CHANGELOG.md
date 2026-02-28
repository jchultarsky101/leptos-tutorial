# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Leptos CSR application that mounts a `Hello, world!` paragraph into the document body.
- Tracing support via `tracing-subscriber` and `tracing-web`, with log level configurable through the `RUST_LOG` environment variable at compile time.
- Cargo workspace layout with three members: `common` (shared library), `ui` (Leptos CSR frontend), and `api` (server-side binary).
- `common` crate: `CatalogPath` newtype with full path validation, `CatalogError` (thiserror), domain models (`FolderEntry`, `FileEntry`), and all REST DTOs with `serde`/`utoipa`/`garde` derives.
- `api` crate: Axum 0.8 REST server with `InMemoryMetadataStore` and `LocalFileStore` backends, OpenAPI documentation via `utoipa` 5 + `utoipa-swagger-ui` 9, all CRUD endpoints for folders and files (create, list, rename, move, delete; upload, download, re-upload, delete).
- CI workflow split into three parallel jobs: `fmt` (rustfmt gate), `check-wasm` (WASM + clippy -D warnings), `check-native` (native + clippy -D warnings + tests).

### Changed

- Upgrade `utoipa-swagger-ui` from 8 to 9 to resolve axum 0.7/0.8 version conflict.
- Rename/move routes use action-first URL prefix (`/folder-rename/{*path}`, `/folder-move/{*path}`, `/file-rename/{*path}`, `/file-move/{*path}`) because `matchit` requires wildcard segments to be terminal.

[Unreleased]: https://github.com/jchultarsky101/leptos-tutorial/commits/main/
