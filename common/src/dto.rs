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

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RenameFolderRequest {
    #[garde(length(min = 1, max = 255))]
    pub new_name: String,
}

/// Path is validated at deserialisation time by [`CatalogPath`]'s custom
/// `Deserialize` impl; no additional garde rules are needed.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveFolderRequest {
    pub new_parent_path: CatalogPath,
}

// ── File request bodies ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RenameFileRequest {
    #[garde(length(min = 1, max = 255))]
    pub new_name: String,
}

/// Path is validated at deserialisation time; no additional garde rules needed.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveFileRequest {
    pub new_folder_path: CatalogPath,
}

// ── Generic error response ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
