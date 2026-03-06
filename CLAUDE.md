Act as a Senior Rust Architect. We are following a zero-panic, TDD-first workflow. Before finalizing any file edit, you must run cargo check and cargo clippy. Do not use placeholders; output only complete, idiomatic Rust. Use thiserror for all custom error types. Acknowledge and summarize your understanding of these guardrails.

# Rust Development Rules

## Performance & Personality
- Act as a Senior Rust Architect.
- Maintain "Dense Mode": Minimize conversational fluff; focus on high-quality, production-ready code.
- No placeholders (e.g., `// ...`): All code must be complete and compilable.

## Safety & Idioms
- **No Panics**: Never use `.unwrap()` or `panic!`. Use `Result` or `Option` with `?` propagation. 
- **Error Handling**: Use [thiserror](https://crates.io) for library errors and [anyhow](https://crates.io) for application-level handling.
- **Ownership**: Strictly follow borrow checker rules. Prefer owned types (`String`, `Vec`) initially, optimizing for references only when necessary for performance.
- **Dependencies**: Consult `Cargo.toml` before adding crates. Prefer `std` over external crates unless strictly necessary.

## Development Workflow (TDD)
1. **Red**: Write a failing test in the `tests/` directory or a `mod tests` block.
2. **Green**: Implement the minimal logic to pass the test.
3. **Refactor**: Run `cargo clippy -- -D warnings` and `cargo fmt` to ensure idiomatic quality.

## Post-Edit Command
After every code change, you MUST run:
`cargo check && cargo clippy -- -D warnings`

---

# Project State (updated 2026-03-06)

## Project: Ember Trove
A file catalog web app. Rust + Leptos 0.8 CSR/WASM frontend, Axum REST backend.
- `api/` — Axum backend, port 3000
- `ui/` — Leptos/Trunk frontend (Tailwind CSS v4)
- `common/` — shared DTOs

## Git Flow
- Persistent branches: `main` and `develop` only.
- Features: `feature/jc/...` branched from `develop`, worked in `.claude/worktrees/<name>/`, merged back with `--no-ff`, worktree + branch deleted after merge.
- **Current state**: `develop` is ahead of `main` with three features. `main` has not been released yet.

## Completed Features (on `develop`, not yet on `main`)

### 1. Dark Mode — STL + Markdown + CSV
- `ui/index.html`: WebGL canvas transparent (`setClearColor(0,0,0,0)`); dark bg driven by CSS.
- `ui/input.css`: `.dark .prose` full override set; `.dark .csv-table` overrides; `.dark .prose pre code { background: transparent }` (fixes inner code block highlight leak).
- `ui/src/components/file_preview.rs`: `dark:bg-gray-800` on STL container/toolbar; `dark:text-gray-200` on prose; dark borders on CSV footer.

### 2. Markdown Editor
- Pencil icon in preview header → opens inline textarea pre-populated with raw Markdown source.
- Save / Cancel buttons replace the pencil while editing.
- Save calls `api::save_file_text()` → `PUT /files/{path}` multipart; updates `committed_content: RwSignal<Option<String>>` immediately; increments `catalog_version` to refresh file list.
- Reset Effect clears editor state on every preview target change.
- `render_markdown` fixed: filters only `Event::Html` blocks (not `Event::InlineHtml`); ammonia builder allows `<input>` for task-list checkboxes.
- 11 unit tests passing in `file_preview.rs`.
- Key files: `ui/Cargo.toml` (added `HtmlTextAreaElement`, `BlobPropertyBag`), `ui/src/api.rs` (`save_file_text`), `ui/src/components/file_preview.rs`.

## Architecture Quick-Reference
- Context signals in `app.rs`: `catalog_version: RwSignal<u32>`, `preview_file: RwSignal<Option<PreviewTarget>>`, `modal: RwSignal<Option<ModalState>>`.
- `PreviewTarget { path: CatalogPath, content_type: String }` — `CatalogPath::name()` = filename, `::as_str()` = full path.
- Dark mode: `dark` class on `<html>`, managed by Leptos signal + `localStorage`. Tailwind v4 variant: `@custom-variant dark (&:where(.dark, .dark *))`.
- Error types: `UiError` (thiserror) in `ui/src/error.rs`; `ApiError` in `api/`.

## Dev Servers
- API: `cargo run -p api` → port 3000
- UI: `cd ui && trunk serve` (or `--port NNNN` for a worktree). Start fresh from the correct directory.
