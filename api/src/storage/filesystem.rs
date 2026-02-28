use std::path::PathBuf;

use bytes::Bytes;
use common::{CatalogError, CatalogPath};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use super::{BoxFuture, FileStore};

/// Stores file content on the local filesystem under a configurable base
/// directory. The virtual catalog path `/docs/report.pdf` is stored at
/// `{base_dir}/docs/report.pdf`.
pub struct LocalFileStore {
    base_dir: PathBuf,
}

impl LocalFileStore {
    /// Create a new store rooted at `base_dir`. The directory is created if
    /// it does not already exist.
    pub async fn new(base_dir: impl Into<PathBuf>) -> Result<Self, CatalogError> {
        let base_dir = base_dir.into();
        tokio::fs::create_dir_all(&base_dir).await?;
        Ok(Self { base_dir })
    }

    /// Convert a virtual [`CatalogPath`] to a physical filesystem path.
    ///
    /// # Security
    /// `CatalogPath` rejects `..` segments at construction time. We also
    /// verify the resolved path starts with `base_dir` as defence-in-depth.
    fn physical_path(&self, path: &CatalogPath) -> Result<PathBuf, CatalogError> {
        let relative = path.as_str().trim_start_matches('/');
        let physical = self.base_dir.join(relative);
        if !physical.starts_with(&self.base_dir) {
            return Err(CatalogError::InvalidPath(
                "path would escape the storage root".into(),
            ));
        }
        Ok(physical)
    }
}

impl FileStore for LocalFileStore {
    fn write<'a>(
        &'a self,
        path: &'a CatalogPath,
        data: Bytes,
    ) -> BoxFuture<'a, Result<u64, CatalogError>> {
        Box::pin(async move {
            let physical = self.physical_path(path)?;
            if let Some(parent) = physical.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let mut file = tokio::fs::File::create(&physical).await?;
            file.write_all(&data).await?;
            file.flush().await?;
            Ok(data.len() as u64)
        })
    }

    fn read<'a>(&'a self, path: &'a CatalogPath) -> BoxFuture<'a, Result<File, CatalogError>> {
        Box::pin(async move {
            let physical = self.physical_path(path)?;
            tokio::fs::File::open(&physical)
                .await
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        CatalogError::FileNotFound(path.as_str().to_owned())
                    }
                    _ => CatalogError::Io(e),
                })
        })
    }

    fn delete<'a>(&'a self, path: &'a CatalogPath) -> BoxFuture<'a, Result<(), CatalogError>> {
        Box::pin(async move {
            let physical = self.physical_path(path)?;
            tokio::fs::remove_file(&physical)
                .await
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        CatalogError::FileNotFound(path.as_str().to_owned())
                    }
                    _ => CatalogError::Io(e),
                })
        })
    }

    fn rename<'a>(
        &'a self,
        old_path: &'a CatalogPath,
        new_path: &'a CatalogPath,
    ) -> BoxFuture<'a, Result<(), CatalogError>> {
        Box::pin(async move {
            let old_physical = self.physical_path(old_path)?;
            let new_physical = self.physical_path(new_path)?;
            if let Some(parent) = new_physical.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::rename(&old_physical, &new_physical)
                .await
                .map_err(CatalogError::Io)
        })
    }
}
