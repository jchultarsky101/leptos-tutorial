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

/// All folder and file entries, returned by [`MetadataStore::search_entries`].
pub type AllEntries = (Vec<FolderEntry>, Vec<FileEntry>);

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

    /// Relocate a folder to `new_path`, which may differ in name, parent, or
    /// both. All descendant folders and files are remapped atomically.
    fn relocate_folder<'a>(
        &'a self,
        old_path: &'a CatalogPath,
        new_path: CatalogPath,
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

    /// Relocate a file to `new_path`, which may differ in name, folder, or
    /// both. The caller is responsible for renaming the physical file content
    /// (via [`FileStore::rename`]) before calling this method.
    fn relocate_file<'a>(
        &'a self,
        old_path: &'a CatalogPath,
        new_path: CatalogPath,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>>;

    // ── Search ────────────────────────────────────────────────────────────

    /// Return all folder and file entries for search filtering.
    fn search_entries<'a>(&'a self) -> BoxFuture<'a, Result<AllEntries, CatalogError>>;

    // ── Statistics ────────────────────────────────────────────────────────

    /// Return aggregate catalog statistics.
    fn stats<'a>(&'a self) -> BoxFuture<'a, Result<common::dto::StatsDto, CatalogError>>;
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
