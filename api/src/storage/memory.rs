use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use common::{
    CatalogError, CatalogPath,
    dto::{FileDto, FolderContentsDto, FolderDto},
    model::{FileEntry, FolderEntry},
};
use tokio::sync::RwLock;

use super::{AllEntries, BoxFuture, MetadataStore, file_to_dto, folder_to_dto};

pub struct InMemoryMetadataStore {
    folders: RwLock<HashMap<CatalogPath, FolderEntry>>,
    files: RwLock<HashMap<CatalogPath, FileEntry>>,
}

impl InMemoryMetadataStore {
    pub fn new() -> Self {
        let root = CatalogPath::new("/").expect("root path is always valid");
        let now = Utc::now().to_rfc3339();
        let root_entry = FolderEntry {
            path: root.clone(),
            created_at: now.clone(),
            modified_at: now,
        };
        let mut folders = HashMap::new();
        folders.insert(root, root_entry);
        Self {
            folders: RwLock::new(folders),
            files: RwLock::new(HashMap::new()),
        }
    }

    /// Walk a filesystem directory and populate the metadata store from what
    /// exists on disk. This re-seeds the in-memory state after a restart so
    /// that files persisted by [`super::filesystem::LocalFileStore`] are
    /// immediately visible in the catalog.
    pub async fn scan_directory(&self, base_dir: &Path) -> Result<(usize, usize), CatalogError> {
        use tokio::fs;

        let mut dir_count: usize = 0;
        let mut file_count: usize = 0;
        let mut stack = vec![base_dir.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let mut entries = fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let relative = path
                    .strip_prefix(base_dir)
                    .map_err(|e| CatalogError::Storage(e.to_string()))?;
                let catalog_str = format!("/{}", relative.to_string_lossy().replace('\\', "/"));
                let catalog_path = match CatalogPath::new(&catalog_str) {
                    Ok(p) => p,
                    Err(_) => continue, // skip unparseable paths
                };

                let metadata = fs::metadata(&path).await?;
                let now = Utc::now().to_rfc3339();

                if metadata.is_dir() {
                    let mut folders = self.folders.write().await;
                    if !folders.contains_key(&catalog_path) {
                        folders.insert(
                            catalog_path.clone(),
                            FolderEntry {
                                path: catalog_path,
                                created_at: now.clone(),
                                modified_at: now,
                            },
                        );
                        dir_count += 1;
                    }
                    drop(folders);
                    stack.push(path);
                } else if metadata.is_file() {
                    let size = metadata.len();
                    let content_type = mime_from_extension(&catalog_str);
                    let mut files = self.files.write().await;
                    if !files.contains_key(&catalog_path) {
                        files.insert(
                            catalog_path.clone(),
                            FileEntry {
                                path: catalog_path,
                                size_bytes: size,
                                content_type,
                                created_at: now.clone(),
                                modified_at: now,
                            },
                        );
                        file_count += 1;
                    }
                    drop(files);
                }
            }
        }
        Ok((dir_count, file_count))
    }
}

/// Guess a MIME type from a file extension.
fn mime_from_extension(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "txt" => "text/plain",
        "csv" => "text/csv",
        "md" | "markdown" => "text/markdown",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "stl" => "model/stl",
        "zip" => "application/zip",
        "toml" => "text/plain",
        "yaml" | "yml" => "text/yaml",
        _ => "application/octet-stream",
    }
    .to_owned()
}

impl Default for InMemoryMetadataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataStore for InMemoryMetadataStore {
    fn create_folder<'a>(
        &'a self,
        path: CatalogPath,
    ) -> BoxFuture<'a, Result<FolderEntry, CatalogError>> {
        Box::pin(async move {
            let parent = path
                .parent()
                .ok_or_else(|| CatalogError::InvalidPath("cannot create root".into()))?;

            let mut guard = self.folders.write().await;
            if !guard.contains_key(&parent) {
                return Err(CatalogError::FolderNotFound(parent.as_str().to_owned()));
            }
            if guard.contains_key(&path) {
                return Err(CatalogError::AlreadyExists(path.as_str().to_owned()));
            }
            let now = Utc::now().to_rfc3339();
            let entry = FolderEntry {
                path: path.clone(),
                created_at: now.clone(),
                modified_at: now,
            };
            guard.insert(path, entry.clone());
            Ok(entry)
        })
    }

    fn delete_folder_recursive<'a>(
        &'a self,
        path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<(), CatalogError>> {
        Box::pin(async move {
            if path.is_root() {
                return Err(CatalogError::CannotDeleteRoot);
            }
            let mut folders = self.folders.write().await;
            if !folders.contains_key(path) {
                return Err(CatalogError::FolderNotFound(path.as_str().to_owned()));
            }
            folders.retain(|k, _| !k.starts_with_folder(path));
            drop(folders);

            let mut files = self.files.write().await;
            files.retain(|k, _| !k.starts_with_folder(path));
            Ok(())
        })
    }

    fn relocate_folder<'a>(
        &'a self,
        old_path: &'a CatalogPath,
        new_path: CatalogPath,
    ) -> BoxFuture<'a, Result<FolderEntry, CatalogError>> {
        Box::pin(async move {
            if old_path.is_root() {
                return Err(CatalogError::InvalidPath("cannot relocate root".into()));
            }
            // Prevent circular moves: new_path must not be inside old_path.
            if new_path.starts_with_folder(old_path) {
                return Err(CatalogError::CircularMove);
            }
            let new_parent = new_path
                .parent()
                .ok_or_else(|| CatalogError::InvalidPath("new path has no parent".into()))?;

            let mut folders = self.folders.write().await;
            if !folders.contains_key(old_path) {
                return Err(CatalogError::FolderNotFound(old_path.as_str().to_owned()));
            }
            if !folders.contains_key(&new_parent) {
                return Err(CatalogError::FolderNotFound(new_parent.as_str().to_owned()));
            }
            if folders.contains_key(&new_path) {
                return Err(CatalogError::AlreadyExists(new_path.as_str().to_owned()));
            }

            // Remap the folder itself and all descendants.
            let affected: Vec<CatalogPath> = folders
                .keys()
                .filter(|k| k.starts_with_folder(old_path))
                .cloned()
                .collect();
            let now = Utc::now().to_rfc3339();
            for old_key in affected {
                if let Some(mut entry) = folders.remove(&old_key) {
                    let new_key = remap_path(&old_key, old_path, &new_path)?;
                    entry.path = new_key.clone();
                    if new_key == new_path {
                        entry.modified_at = now.clone();
                    }
                    folders.insert(new_key, entry);
                }
            }
            drop(folders);

            // Remap all files whose paths are under old_path.
            let mut files = self.files.write().await;
            let affected_files: Vec<CatalogPath> = files
                .keys()
                .filter(|k| k.starts_with_folder(old_path))
                .cloned()
                .collect();
            for old_key in affected_files {
                if let Some(mut entry) = files.remove(&old_key) {
                    let new_key = remap_path(&old_key, old_path, &new_path)?;
                    entry.path = new_key.clone();
                    files.insert(new_key, entry);
                }
            }
            drop(files);

            let folders = self.folders.read().await;
            folders
                .get(&new_path)
                .cloned()
                .ok_or_else(|| CatalogError::FolderNotFound(new_path.as_str().to_owned()))
        })
    }

    fn list_folder<'a>(
        &'a self,
        path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<FolderContentsDto, CatalogError>> {
        Box::pin(async move {
            let folders = self.folders.read().await;
            if !folders.contains_key(path) {
                return Err(CatalogError::FolderNotFound(path.as_str().to_owned()));
            }
            let child_folders: Vec<FolderDto> = folders
                .values()
                .filter(|e| e.path.parent().as_ref() == Some(path))
                .map(folder_to_dto)
                .collect();
            drop(folders);

            let files = self.files.read().await;
            let child_files: Vec<FileDto> = files
                .values()
                .filter(|e| e.path.parent().as_ref() == Some(path))
                .map(file_to_dto)
                .collect();
            drop(files);

            Ok(FolderContentsDto {
                path: path.clone(),
                folders: child_folders,
                files: child_files,
            })
        })
    }

    fn create_file_entry<'a>(
        &'a self,
        entry: FileEntry,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>> {
        Box::pin(async move {
            let parent = entry
                .path
                .parent()
                .ok_or_else(|| CatalogError::InvalidPath("file has no parent folder".into()))?;
            let mut files = self.files.write().await;
            let folders = self.folders.read().await;
            if !folders.contains_key(&parent) {
                return Err(CatalogError::FolderNotFound(parent.as_str().to_owned()));
            }
            if files.contains_key(&entry.path) {
                return Err(CatalogError::AlreadyExists(entry.path.as_str().to_owned()));
            }
            files.insert(entry.path.clone(), entry.clone());
            Ok(entry)
        })
    }

    fn get_file_entry<'a>(
        &'a self,
        path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>> {
        Box::pin(async move {
            let guard = self.files.read().await;
            guard
                .get(path)
                .cloned()
                .ok_or_else(|| CatalogError::FileNotFound(path.as_str().to_owned()))
        })
    }

    fn update_file_entry<'a>(
        &'a self,
        entry: FileEntry,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>> {
        Box::pin(async move {
            let mut guard = self.files.write().await;
            if !guard.contains_key(&entry.path) {
                return Err(CatalogError::FileNotFound(entry.path.as_str().to_owned()));
            }
            guard.insert(entry.path.clone(), entry.clone());
            Ok(entry)
        })
    }

    fn delete_file_entry<'a>(
        &'a self,
        path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<(), CatalogError>> {
        Box::pin(async move {
            let mut guard = self.files.write().await;
            if guard.remove(path).is_none() {
                return Err(CatalogError::FileNotFound(path.as_str().to_owned()));
            }
            Ok(())
        })
    }

    fn relocate_file<'a>(
        &'a self,
        old_path: &'a CatalogPath,
        new_path: CatalogPath,
    ) -> BoxFuture<'a, Result<FileEntry, CatalogError>> {
        Box::pin(async move {
            let new_parent = new_path
                .parent()
                .ok_or_else(|| CatalogError::InvalidPath("new path has no parent".into()))?;

            let folders = self.folders.read().await;
            if !folders.contains_key(&new_parent) {
                return Err(CatalogError::FolderNotFound(new_parent.as_str().to_owned()));
            }
            drop(folders);

            let mut guard = self.files.write().await;
            let mut entry = guard
                .remove(old_path)
                .ok_or_else(|| CatalogError::FileNotFound(old_path.as_str().to_owned()))?;
            if guard.contains_key(&new_path) {
                // Rollback: re-insert at the original key.
                guard.insert(old_path.clone(), entry);
                return Err(CatalogError::AlreadyExists(new_path.as_str().to_owned()));
            }
            entry.path = new_path.clone();
            entry.modified_at = Utc::now().to_rfc3339();
            guard.insert(new_path, entry.clone());
            Ok(entry)
        })
    }

    fn search_entries<'a>(&'a self) -> BoxFuture<'a, Result<AllEntries, CatalogError>> {
        Box::pin(async move {
            let folders = self.folders.read().await;
            let files = self.files.read().await;
            Ok((
                folders.values().cloned().collect(),
                files.values().cloned().collect(),
            ))
        })
    }

    fn stats<'a>(&'a self) -> BoxFuture<'a, Result<common::dto::StatsDto, CatalogError>> {
        Box::pin(async move {
            use std::collections::BTreeMap;

            let folders = self.folders.read().await;
            // Exclude the implicit root folder from the count.
            let total_folders = folders.values().filter(|e| !e.path.is_root()).count() as u64;
            drop(folders);

            let files = self.files.read().await;
            let total_files = files.len() as u64;
            let total_size_bytes: u64 = files.values().map(|e| e.size_bytes).sum();

            // Bucket uploads by calendar day (UTC). The `created_at` field is
            // RFC 3339 — the first 10 bytes are always "YYYY-MM-DD".
            let mut by_day: BTreeMap<String, u64> = BTreeMap::new();
            for entry in files.values() {
                let day: String = entry.created_at.chars().take(10).collect();
                *by_day.entry(day).or_insert(0) += 1;
            }
            drop(files);

            let uploads_by_day = by_day
                .into_iter()
                .map(|(date, count)| common::dto::DayCount { date, count })
                .collect();

            Ok(common::dto::StatsDto {
                total_files,
                total_folders,
                total_size_bytes,
                uploads_by_day,
            })
        })
    }
}

/// Rewrites a path by replacing the `old_prefix` component with `new_prefix`.
fn remap_path(
    path: &CatalogPath,
    old_prefix: &CatalogPath,
    new_prefix: &CatalogPath,
) -> Result<CatalogPath, CatalogError> {
    let old_str = old_prefix.as_str();
    let path_str = path.as_str();
    let suffix = path_str.strip_prefix(old_str).ok_or_else(|| {
        CatalogError::Storage(format!("path {path_str} does not start with {old_str}"))
    })?;
    let new_str = format!("{}{}", new_prefix.as_str(), suffix);
    CatalogPath::new(&new_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> InMemoryMetadataStore {
        InMemoryMetadataStore::new()
    }

    #[tokio::test]
    async fn root_exists_on_init() {
        let s = store();
        let root = CatalogPath::new("/").unwrap();
        // Listing the root folder succeeds iff it was seeded correctly.
        let contents = s.list_folder(&root).await.unwrap();
        assert_eq!(contents.path, root);
    }

    #[tokio::test]
    async fn create_and_list_folder() {
        let s = store();
        let path = CatalogPath::new("/docs").unwrap();
        s.create_folder(path.clone()).await.unwrap();

        let root = CatalogPath::new("/").unwrap();
        let contents = s.list_folder(&root).await.unwrap();
        assert_eq!(contents.folders.len(), 1);
        assert_eq!(contents.folders[0].path, path);
    }

    #[tokio::test]
    async fn create_duplicate_folder_errors() {
        let s = store();
        let path = CatalogPath::new("/docs").unwrap();
        s.create_folder(path.clone()).await.unwrap();
        let result = s.create_folder(path).await;
        assert!(matches!(result, Err(CatalogError::AlreadyExists(_))));
    }

    #[tokio::test]
    async fn delete_folder_removes_children() {
        let s = store();
        let docs = CatalogPath::new("/docs").unwrap();
        let reports = CatalogPath::new("/docs/reports").unwrap();
        s.create_folder(docs.clone()).await.unwrap();
        s.create_folder(reports).await.unwrap();
        s.delete_folder_recursive(&docs).await.unwrap();

        let root = CatalogPath::new("/").unwrap();
        let contents = s.list_folder(&root).await.unwrap();
        assert!(contents.folders.is_empty());
    }

    #[tokio::test]
    async fn delete_root_errors() {
        let s = store();
        let root = CatalogPath::new("/").unwrap();
        let result = s.delete_folder_recursive(&root).await;
        assert!(matches!(result, Err(CatalogError::CannotDeleteRoot)));
    }

    #[tokio::test]
    async fn relocate_folder_rename_only() {
        let s = store();
        let docs = CatalogPath::new("/docs").unwrap();
        s.create_folder(docs.clone()).await.unwrap();
        let new_path = CatalogPath::new("/documents").unwrap();
        let entry = s.relocate_folder(&docs, new_path.clone()).await.unwrap();
        assert_eq!(entry.path, new_path);
        // Old path must no longer appear in root listing.
        let root = CatalogPath::new("/").unwrap();
        let contents = s.list_folder(&root).await.unwrap();
        assert!(contents.folders.iter().all(|f| f.path != docs));
    }

    #[tokio::test]
    async fn relocate_folder_move_and_rename() {
        let s = store();
        let docs = CatalogPath::new("/docs").unwrap();
        let archive = CatalogPath::new("/archive").unwrap();
        s.create_folder(docs.clone()).await.unwrap();
        s.create_folder(archive.clone()).await.unwrap();
        // Move /docs → /archive and rename simultaneously.
        let target = CatalogPath::new("/archive/old-docs").unwrap();
        let entry = s.relocate_folder(&docs, target.clone()).await.unwrap();
        assert_eq!(entry.path, target);
        // Verify /archive/old-docs appears in /archive listing.
        let contents = s.list_folder(&archive).await.unwrap();
        assert!(contents.folders.iter().any(|f| f.path == target));
    }

    #[tokio::test]
    async fn relocate_folder_remaps_children() {
        let s = store();
        let docs = CatalogPath::new("/docs").unwrap();
        let child = CatalogPath::new("/docs/reports").unwrap();
        s.create_folder(docs.clone()).await.unwrap();
        s.create_folder(child).await.unwrap();
        let target = CatalogPath::new("/documentation").unwrap();
        s.relocate_folder(&docs, target.clone()).await.unwrap();
        // Child must be accessible under the new parent.
        let expected_child = CatalogPath::new("/documentation/reports").unwrap();
        let contents = s.list_folder(&target).await.unwrap();
        assert!(contents.folders.iter().any(|f| f.path == expected_child));
    }

    #[tokio::test]
    async fn relocate_folder_circular_errors() {
        let s = store();
        let docs = CatalogPath::new("/docs").unwrap();
        let sub = CatalogPath::new("/docs/sub").unwrap();
        s.create_folder(docs.clone()).await.unwrap();
        s.create_folder(sub.clone()).await.unwrap();
        // Attempting to move /docs into /docs/sub/docs is circular.
        let target = CatalogPath::new("/docs/sub/docs").unwrap();
        let result = s.relocate_folder(&docs, target).await;
        assert!(matches!(result, Err(CatalogError::CircularMove)));
    }

    #[tokio::test]
    async fn relocate_file_move_and_rename() {
        let s = store();
        let docs = CatalogPath::new("/docs").unwrap();
        let archive = CatalogPath::new("/archive").unwrap();
        s.create_folder(docs.clone()).await.unwrap();
        s.create_folder(archive.clone()).await.unwrap();

        let file_path = CatalogPath::new("/docs/report.txt").unwrap();
        let now = Utc::now().to_rfc3339();
        s.create_file_entry(FileEntry {
            path: file_path.clone(),
            size_bytes: 0,
            content_type: "text/plain".into(),
            created_at: now.clone(),
            modified_at: now,
        })
        .await
        .unwrap();

        let target = CatalogPath::new("/archive/final-report.txt").unwrap();
        let entry = s.relocate_file(&file_path, target.clone()).await.unwrap();
        assert_eq!(entry.path, target);
    }

    #[tokio::test]
    async fn relocate_file_conflicts_with_existing() {
        let s = store();
        let docs = CatalogPath::new("/docs").unwrap();
        s.create_folder(docs.clone()).await.unwrap();

        let now = Utc::now().to_rfc3339();
        for name in &["a.txt", "b.txt"] {
            let p = docs.join(name).unwrap();
            s.create_file_entry(FileEntry {
                path: p,
                size_bytes: 0,
                content_type: "text/plain".into(),
                created_at: now.clone(),
                modified_at: now.clone(),
            })
            .await
            .unwrap();
        }

        let from = docs.join("a.txt").unwrap();
        let to = docs.join("b.txt").unwrap();
        let result = s.relocate_file(&from, to).await;
        assert!(matches!(result, Err(CatalogError::AlreadyExists(_))));
    }
}
