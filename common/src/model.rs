use serde::{Deserialize, Serialize};

use crate::path::CatalogPath;

/// Internal representation of a folder stored in the metadata store.
/// Clients receive [`crate::dto::FolderDto`] instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub path: CatalogPath,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub modified_at: String,
}

/// Internal representation of a file stored in the metadata store.
/// Clients receive [`crate::dto::FileDto`] instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: CatalogPath,
    pub size_bytes: u64,
    pub content_type: String,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub modified_at: String,
}
