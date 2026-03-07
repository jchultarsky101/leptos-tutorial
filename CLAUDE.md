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

# Project State (updated 2026-03-07)

## Project: Ember Trove
A file catalog web app. Rust + Leptos 0.8 CSR/WASM frontend, Axum REST backend.
- `api/` — Axum backend, port 3000
- `ui/` — Leptos/Trunk frontend (Tailwind CSS v4)
- `common/` — shared DTOs

## Git Flow
- Persistent branches: `main` and `develop` only.
- Features: `feature/jc/...` branched from `develop`, worked in `.claude/worktrees/<name>/`, merged back with `--no-ff`, worktree + branch deleted after merge.
- **Current state**: `main` == `develop` == `v0.3.0`. No unreleased work in flight.

## Released Features (on `main`, tagged `v0.3.0`)

### 1. Dark Mode — STL + Markdown + CSV
- `ui/index.html`: WebGL canvas transparent (`setClearColor(0,0,0,0)`); dark bg driven by CSS.
- `ui/input.css`: `.dark .prose` full override set; `.dark .csv-table` overrides; `.dark .prose pre code { background: transparent }` (fixes inner code block highlight leak).
- `ui/src/components/file_preview.rs`: `dark:bg-gray-800` on STL container/toolbar; `dark:text-gray-200` on prose; dark borders on CSV footer.

### 2. Markdown Inline Editor
- Pencil icon in preview header → opens inline textarea pre-populated with raw Markdown source.
- Save / Cancel buttons replace the pencil while editing.
- Save calls `api::save_file_text()` → `PUT /files/{path}` multipart; updates `committed_content: RwSignal<Option<String>>` immediately; increments `catalog_version` to refresh file list.
- Reset Effect clears editor state on every preview target change.
- `render_markdown` fixed: filters only `Event::Html` blocks (not `Event::InlineHtml`); ammonia builder allows `<input>` for task-list checkboxes.

### 3. Text + JSON Inline Editor
- `PreviewKind::Json` split from `PreviewKind::Text`; `classify()` checks MIME (`*json*`) or `.json` extension before the generic `text/` block.
- `prettify_json(input: &str) -> String` — pretty-prints valid JSON on editor open; passes through invalid JSON unchanged.
- `validate_json(input: &str) -> Result<(), String>` — blocks save and surfaces an inline error banner for malformed JSON.
- `api::save_file_text` takes `content_type: &str`; correct MIME forwarded on `PUT /files/{path}`.
- `is_editable()` replaces `is_markdown()` — edit pencil visible for Markdown, Text, and Json kinds.
- Read-only `<pre>` container uses `dark:bg-gray-800` (matches all other panel cards).
- 18 unit tests passing in `file_preview.rs` (7 new: `classify_json_*`, `prettify_json_*`, `validate_json_*`).

## Architecture Quick-Reference
- Context signals in `app.rs`: `catalog_version: RwSignal<u32>`, `preview_file: RwSignal<Option<PreviewTarget>>`, `modal: RwSignal<Option<ModalState>>`.
- `PreviewTarget { path: CatalogPath, content_type: String }` — `CatalogPath::name()` = filename, `::as_str()` = full path.
- Dark mode: `dark` class on `<html>`, managed by Leptos signal + `localStorage`. Tailwind v4 variant: `@custom-variant dark (&:where(.dark, .dark *))`.
- `PreviewKind` variants: `Image | Markdown | Text | Json | Csv | Stl | Unsupported` — classified in `classify(content_type, filename)`.
- Error types: `UiError` (thiserror) in `ui/src/error.rs`; `ApiError` in `api/`.
- Editor pattern: `is_editing: RwSignal<bool>`, `edit_content: RwSignal<String>`, `committed_content: RwSignal<Option<String>>`, `saving: RwSignal<bool>`, `save_error: RwSignal<Option<String>>`.
- Reactive closure gotcha: closures inside `view!` must be `Fn`, not `FnOnce`. Pre-clone any `String` captured by a `move` closure before the closure block.

## Key Files
- `ui/src/components/file_preview.rs` — all preview + editor logic; 18 tests in `mod tests`
- `ui/src/api.rs` — `save_file_text(path, content, content_type)`, `fetch_file_text()`, etc.
- `ui/src/app.rs` — context signals, dark mode toggle
- `ui/input.css` — Tailwind v4 source; dark mode overrides for prose/CSV/code
- `api/src/` — Axum handlers; `PUT /files/{path}` accepts multipart with `file` field

## Dev Servers
- API: `cargo run -p api` → port 3000
- UI: `cd ui && trunk serve` (or `--port NNNN` for a worktree). Start fresh from the correct directory.
- Worktree UI servers must be started from `<worktree>/ui/`, not the repo root.
- Test dark mode in browser: `localStorage.setItem('theme','dark'); document.documentElement.classList.add('dark')`
