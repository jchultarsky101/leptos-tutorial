use garde::Validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    model::{FileEntry, FolderEntry},
    path::CatalogPath,
};

// ── Folder responses ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FolderDto {
    pub path: CatalogPath,
    pub created_at: String,
    pub modified_at: String,
}

impl From<FolderEntry> for FolderDto {
    fn from(e: FolderEntry) -> Self {
        Self {
            path: e.path,
            created_at: e.created_at,
            modified_at: e.modified_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FolderContentsDto {
    pub path: CatalogPath,
    pub folders: Vec<FolderDto>,
    pub files: Vec<FileDto>,
}

// ── File response ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileDto {
    pub path: CatalogPath,
    pub size_bytes: u64,
    pub content_type: String,
    pub created_at: String,
    pub modified_at: String,
}

impl From<FileEntry> for FileDto {
    fn from(e: FileEntry) -> Self {
        Self {
            path: e.path,
            size_bytes: e.size_bytes,
            content_type: e.content_type,
            created_at: e.created_at,
            modified_at: e.modified_at,
        }
    }
}

// ── Folder request bodies ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateFolderRequest {
    #[garde(length(min = 1, max = 255))]
    pub name: String,
}

/// Body for `PATCH /folders/{path}`.
///
/// At least one field must be `Some`. Supplying both renames and moves the
/// folder in a single atomic operation.
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct PatchFolderRequest {
    /// New name for the folder (1–255 chars). Omit to keep the current name.
    #[garde(inner(length(min = 1, max = 255)))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New parent path. Omit to keep the current parent.
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_parent_path: Option<CatalogPath>,
}

// ── File request bodies ───────────────────────────────────────────────────────

/// Body for `PATCH /files/{path}`.
///
/// At least one field must be `Some`. Supplying both renames and moves the
/// file in a single atomic operation.
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct PatchFileRequest {
    /// New name for the file (1–255 chars). Omit to keep the current name.
    #[garde(inner(length(min = 1, max = 255)))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New parent folder path. Omit to keep the current folder.
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_folder_path: Option<CatalogPath>,
}

// ── Search ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResultDto {
    pub path: CatalogPath,
    /// `"file"` or `"folder"`.
    pub kind: String,
    /// Display name (last path segment).
    pub name: String,
    /// For files: size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// For files: MIME type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// `"name"`, `"content"`, or `"both"`.
    pub match_source: String,
    /// Context snippet for content matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResultsDto {
    pub query: String,
    pub fuzzy: bool,
    pub results: Vec<SearchResultDto>,
}

// ── Generic error response ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
