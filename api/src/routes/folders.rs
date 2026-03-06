use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use common::{
    CatalogPath,
    dto::{CreateFolderRequest, FolderDto, PatchFolderRequest},
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

/// Rename and/or move a folder in one atomic operation.
///
/// Supply `name` to rename, `new_parent_path` to move, or both to do both
/// simultaneously. At least one field must be present.
#[utoipa::path(
    patch,
    path = "/folders/{path}",
    params(("path" = String, Path, description = "Folder path (without leading slash)")),
    request_body = common::dto::PatchFolderRequest,
    responses(
        (status = 200, description = "Updated",          body = common::dto::FolderDto),
        (status = 404, description = "Not found",        body = common::dto::ErrorResponse),
        (status = 409, description = "Name taken",       body = common::dto::ErrorResponse),
        (status = 422, description = "Validation error", body = common::dto::ErrorResponse),
    ),
    tag = "folders"
)]
pub async fn patch_folder(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Json(body): Json<PatchFolderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::from(common::CatalogError::Validation(e.to_string())))?;
    if body.name.is_none() && body.new_parent_path.is_none() {
        return Err(ApiError::from(common::CatalogError::Validation(
            "at least one of 'name' or 'new_parent_path' must be provided".into(),
        )));
    }

    let path = decode_path(&raw)?;

    // Determine the target name (supplied or current).
    let new_name = body
        .name
        .as_deref()
        .unwrap_or_else(|| path.name())
        .to_owned();

    // Determine the target parent (supplied or current).
    let new_parent = match body.new_parent_path {
        Some(p) => p,
        None => path.parent().ok_or_else(|| {
            ApiError::from(common::CatalogError::InvalidPath(
                "root has no parent".into(),
            ))
        })?,
    };

    let new_path = new_parent.join(&new_name).map_err(ApiError::from)?;
    if new_path == path {
        return Err(ApiError::from(common::CatalogError::Validation(
            "no changes: name and parent are both unchanged".into(),
        )));
    }

    let entry = state.metadata.relocate_folder(&path, new_path).await?;
    Ok(Json(FolderDto::from(entry)))
}
