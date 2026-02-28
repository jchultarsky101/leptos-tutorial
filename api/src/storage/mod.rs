use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use common::{
    CatalogError, CatalogPath,
    dto::{FileDto, FolderContentsDto, FolderDto},
    model::{FileEntry, FolderEntry},
};
use tokio::fs::File;

pub mod filesystem;
pub mod memory;

/// Convenience alias for a heap-allocated, pinned, `Send`-capable async future.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Async metadata operations over folders and file records.
///
/// Implementations must be `Send + Sync + 'static` so they can be stored in
/// `Arc<dyn MetadataStore>` and shared across Axum handler tasks.
pub trait MetadataStore: Send + Sync + 'static {
    // ── Folder operations ─────────────────────────────────────────────────

    fn create_folder<'a>(
        &'a self,
        path: CatalogPath,
    ) -> BoxFuture<'a, Result<FolderEntry, CatalogError>>;

    fn delete_folder_recursive<'a>(
        &'a self,
        path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<(), CatalogError>>;

    fn rename_folder<'a>(
        &'a self,
        path: &'a CatalogPath,
        new_name: &'a str,
    ) -> BoxFuture<'a, Result<FolderEntry, CatalogError>>;

    fn move_folder<'a>(
        &'a self,
        path: &'a CatalogPath,
        new_parent: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<FolderEntry, CatalogError>>;

    fn list_folder<'a>(
        &'a self,
        path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<FolderContentsDto, CatalogError>>;

    // ── File metadata operations ──────────────────────────────────────────

    fn create_file_entry<'a>(
        &'a self,
        entry: FileEntry,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>>;

    fn get_file_entry<'a>(
        &'a self,
        path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>>;

    fn update_file_entry<'a>(
        &'a self,
        entry: FileEntry,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>>;

    fn delete_file_entry<'a>(
        &'a self,
        path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<(), CatalogError>>;

    fn rename_file<'a>(
        &'a self,
        path: &'a CatalogPath,
        new_name: &'a str,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>>;

    fn move_file<'a>(
        &'a self,
        path: &'a CatalogPath,
        new_folder: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>>;
}

/// Async raw byte storage, separate from metadata.
///
/// The split allows swapping metadata (e.g. SQLite) and file backends
/// (e.g. S3) independently.
pub trait FileStore: Send + Sync + 'static {
    /// Write `data` to the given path and return the number of bytes written.
    fn write<'a>(
        &'a self,
        path: &'a CatalogPath,
        data: Bytes,
    ) -> BoxFuture<'a, Result<u64, CatalogError>>;

    /// Open the file at `path` for streaming.
    fn read<'a>(&'a self, path: &'a CatalogPath) -> BoxFuture<'a, Result<File, CatalogError>>;

    fn delete<'a>(&'a self, path: &'a CatalogPath) -> BoxFuture<'a, Result<(), CatalogError>>;

    fn rename<'a>(
        &'a self,
        old_path: &'a CatalogPath,
        new_path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<(), CatalogError>>;
}

// ── Conversion helpers shared by both storage implementations ────────────────

pub(crate) fn folder_to_dto(entry: &FolderEntry) -> FolderDto {
    FolderDto {
        path: entry.path.clone(),
        created_at: entry.created_at.clone(),
        modified_at: entry.modified_at.clone(),
    }
}

pub(crate) fn file_to_dto(entry: &FileEntry) -> FileDto {
    FileDto {
        path: entry.path.clone(),
        size_bytes: entry.size_bytes,
        content_type: entry.content_type.clone(),
        created_at: entry.created_at.clone(),
        modified_at: entry.modified_at.clone(),
    }
}
