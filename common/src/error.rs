use thiserror::Error;

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("folder not found: {0}")]
    FolderNotFound(String),

    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("cannot delete the root folder")]
    CannotDeleteRoot,

    #[error("destination is a descendant of source")]
    CircularMove,

    #[error("validation error: {0}")]
    Validation(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
