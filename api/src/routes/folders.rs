use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use common::{
    CatalogPath,
    dto::{CreateFolderRequest, MoveFolderRequest, RenameFolderRequest},
};
use garde::Validate;

use crate::{error::ApiError, state::AppState};

// ── Helper ────────────────────────────────────────────────────────────────────

/// Decode a wildcard path segment and prepend the leading `/`.
fn decode_path(raw: &str) -> Result<CatalogPath, ApiError> {
    CatalogPath::new(&format!("/{raw}")).map_err(ApiError::from)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List the contents of the root folder.
#[utoipa::path(
    get,
    path = "/folders",
    responses(
        (status = 200, description = "Root folder contents", body = common::dto::FolderContentsDto),
        (status = 500, description = "Storage error",        body = common::dto::ErrorResponse),
    ),
    tag = "folders"
)]
pub async fn list_root(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let root = CatalogPath::new("/").map_err(ApiError::from)?;
    let contents = state.metadata.list_folder(&root).await?;
    Ok(Json(contents))
}

/// List the contents of a folder by path.
#[utoipa::path(
    get,
    path = "/folders/{path}",
    params(("path" = String, Path, description = "Folder path (without leading slash)")),
    responses(
        (status = 200, description = "Folder contents", body = common::dto::FolderContentsDto),
        (status = 404, description = "Not found",       body = common::dto::ErrorResponse),
        (status = 422, description = "Invalid path",    body = common::dto::ErrorResponse),
    ),
    tag = "folders"
)]
pub async fn list_folder(
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let path = decode_path(&raw)?;
    let contents = state.metadata.list_folder(&path).await?;
    Ok(Json(contents))
}

/// Create a subfolder under the folder identified by `path`.
#[utoipa::path(
    post,
    path = "/folders/{path}",
    params(("path" = String, Path, description = "Parent folder path")),
    request_body = common::dto::CreateFolderRequest,
    responses(
        (status = 201, description = "Folder created", body = common::dto::FolderDto),
        (status = 404, description = "Parent not found",body = common::dto::ErrorResponse),
        (status = 409, description = "Already exists", body = common::dto::ErrorResponse),
        (status = 422, description = "Validation error",body = common::dto::ErrorResponse),
    ),
    tag = "folders"
)]
pub async fn create_folder(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Json(body): Json<CreateFolderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::from(common::CatalogError::Validation(e.to_string())))?;
    let parent = decode_path(&raw)?;
    let new_path = parent.join(&body.name).map_err(ApiError::from)?;
    let entry = state.metadata.create_folder(new_path).await?;
    Ok((
        StatusCode::CREATED,
        Json(common::dto::FolderDto::from(entry)),
    ))
}

/// Create a subfolder directly under root.
#[utoipa::path(
    post,
    path = "/folders",
    request_body = common::dto::CreateFolderRequest,
    responses(
        (status = 201, description = "Folder created", body = common::dto::FolderDto),
        (status = 409, description = "Already exists", body = common::dto::ErrorResponse),
        (status = 422, description = "Validation error",body = common::dto::ErrorResponse),
    ),
    tag = "folders"
)]
pub async fn create_folder_in_root(
    State(state): State<AppState>,
    Json(body): Json<CreateFolderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::from(common::CatalogError::Validation(e.to_string())))?;
    let root = CatalogPath::new("/").map_err(ApiError::from)?;
    let new_path = root.join(&body.name).map_err(ApiError::from)?;
    let entry = state.metadata.create_folder(new_path).await?;
    Ok((
        StatusCode::CREATED,
        Json(common::dto::FolderDto::from(entry)),
    ))
}

/// Delete a folder and all of its contents recursively.
#[utoipa::path(
    delete,
    path = "/folders/{path}",
    params(("path" = String, Path, description = "Folder path")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Cannot delete root", body = common::dto::ErrorResponse),
        (status = 404, description = "Not found",          body = common::dto::ErrorResponse),
    ),
    tag = "folders"
)]
pub async fn delete_folder(
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let path = decode_path(&raw)?;
    state.metadata.delete_folder_recursive(&path).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Rename a folder in place.
#[utoipa::path(
    patch,
    path = "/folder-rename/{path}",
    params(("path" = String, Path, description = "Folder path (without leading slash)")),
    request_body = common::dto::RenameFolderRequest,
    responses(
        (status = 200, description = "Renamed",          body = common::dto::FolderDto),
        (status = 404, description = "Not found",        body = common::dto::ErrorResponse),
        (status = 409, description = "Name taken",       body = common::dto::ErrorResponse),
        (status = 422, description = "Validation error", body = common::dto::ErrorResponse),
    ),
    tag = "folders"
)]
pub async fn rename_folder(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Json(body): Json<RenameFolderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::from(common::CatalogError::Validation(e.to_string())))?;
    let path = decode_path(&raw)?;
    let entry = state.metadata.rename_folder(&path, &body.new_name).await?;
    Ok(Json(common::dto::FolderDto::from(entry)))
}

/// Move a folder (and its entire subtree) under a new parent.
#[utoipa::path(
    patch,
    path = "/folder-move/{path}",
    params(("path" = String, Path, description = "Folder path (without leading slash)")),
    request_body = common::dto::MoveFolderRequest,
    responses(
        (status = 200, description = "Moved",            body = common::dto::FolderDto),
        (status = 404, description = "Not found",        body = common::dto::ErrorResponse),
        (status = 409, description = "Name taken",       body = common::dto::ErrorResponse),
        (status = 422, description = "Invalid move",     body = common::dto::ErrorResponse),
    ),
    tag = "folders"
)]
pub async fn move_folder(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Json(body): Json<MoveFolderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let path = decode_path(&raw)?;
    let entry = state
        .metadata
        .move_folder(&path, &body.new_parent_path)
        .await?;
    Ok(Json(common::dto::FolderDto::from(entry)))
}
