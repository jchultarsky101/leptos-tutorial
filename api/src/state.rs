use std::sync::Arc;

use crate::storage::{FileStore, MetadataStore};

/// Shared application state injected into every Axum handler.
///
/// Both fields use `Arc<dyn Trait>` so the concrete storage implementations
/// can be swapped (e.g. from in-memory to SQLite) without touching handler
/// signatures or the router.
#[derive(Clone)]
pub struct AppState {
    pub metadata: Arc<dyn MetadataStore>,
    pub files: Arc<dyn FileStore>,
}
