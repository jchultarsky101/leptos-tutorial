# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Leptos CSR application that mounts a `Hello, world!` paragraph into the document body.
- Tracing support via `tracing-subscriber` and `tracing-web`, with log level configurable through the `RUST_LOG` environment variable at compile time.
- Cargo workspace layout with three members: `common` (shared library), `ui` (Leptos CSR frontend), and `api` (server-side binary).

[Unreleased]: https://github.com/jchultarsky101/leptos-tutorial/commits/main/
