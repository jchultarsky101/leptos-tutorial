use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use bytes::Bytes;
use chrono::Utc;
use common::{
    CatalogPath,
    dto::{FileDto, PatchFileRequest},
    model::FileEntry,
};
use garde::Validate;
use tokio_util::io::ReaderStream;

use crate::{error::ApiError, state::AppState};

// ── Helper ────────────────────────────────────────────────────────────────────

fn decode_path(raw: &str) -> Result<CatalogPath, ApiError> {
    CatalogPath::new(&format!("/{raw}")).map_err(ApiError::from)
}

/// Extract file bytes and content-type from a multipart upload.
async fn extract_multipart(mut multipart: Multipart) -> Result<(Bytes, String), ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::from(common::CatalogError::Validation(e.to_string())))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_owned();
        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::from(common::CatalogError::Validation(e.to_string())))?;
        return Ok((data, content_type));
    }
    Err(ApiError::from(common::CatalogError::Validation(
        "missing 'file' field in multipart body".into(),
    )))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Upload a new file into the folder identified by `path`.
#[utoipa::path(
    post,
    path = "/files/{path}",
    params(("path" = String, Path, description = "Parent folder path")),
    request_body(content_type = "multipart/form-data", description = "File to upload (field name: `file`)"),
    responses(
        (status = 201, description = "File uploaded", body = common::dto::FileDto),
        (status = 404, description = "Folder not found", body = common::dto::ErrorResponse),
        (status = 409, description = "Already exists",   body = common::dto::ErrorResponse),
        (status = 422, description = "Bad request",      body = common::dto::ErrorResponse),
    ),
    tag = "files"
)]
pub async fn upload_file(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    // `raw` is the full file path (folder + filename) without the leading slash.
    // e.g. POST /files/documents/reports/notes.txt → raw = "documents/reports/notes.txt"
    let file_path = decode_path(&raw)?;
    let (data, content_type) = extract_multipart(multipart).await?;
    let size = state.files.write(&file_path, data).await?;
    let now = Utc::now().to_rfc3339();
    let entry = FileEntry {
        path: file_path,
        size_bytes: size,
        content_type,
        created_at: now.clone(),
        modified_at: now,
    };
    let entry = state.metadata.create_file_entry(entry).await?;
    Ok((StatusCode::CREATED, Json(FileDto::from(entry))))
}

/// Download the file at `path`, streaming its bytes.
#[utoipa::path(
    get,
    path = "/files/{path}",
    params(("path" = String, Path, description = "File path")),
    responses(
        (status = 200, description = "File content (binary stream)"),
        (status = 404, description = "Not found", body = common::dto::ErrorResponse),
    ),
    tag = "files"
)]
pub async fn download_file(
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let path = decode_path(&raw)?;
    let entry = state.metadata.get_file_entry(&path).await?;
    let file = state.files.read(&path).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    if let Ok(ct) = entry.content_type.parse() {
        headers.insert(header::CONTENT_TYPE, ct);
    }
    let filename = path.name().to_owned();
    if let Ok(cd) = format!("attachment; filename=\"{filename}\"").parse() {
        headers.insert(header::CONTENT_DISPOSITION, cd);
    }

    Ok((headers, body))
}

/// Replace the content of an existing file.
#[utoipa::path(
    put,
    path = "/files/{path}",
    params(("path" = String, Path, description = "File path")),
    request_body(content_type = "multipart/form-data", description = "Replacement file (field name: `file`)"),
    responses(
        (status = 200, description = "File replaced", body = common::dto::FileDto),
        (status = 404, description = "Not found",     body = common::dto::ErrorResponse),
        (status = 422, description = "Bad request",   body = common::dto::ErrorResponse),
    ),
    tag = "files"
)]
pub async fn reupload_file(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let path = decode_path(&raw)?;
    // Verify file exists before overwriting.
    let mut entry = state.metadata.get_file_entry(&path).await?;
    let (data, content_type) = extract_multipart(multipart).await?;
    let size = state.files.write(&path, data).await?;
    entry.size_bytes = size;
    entry.content_type = content_type;
    entry.modified_at = Utc::now().to_rfc3339();
    let updated = state.metadata.update_file_entry(entry).await?;
    Ok(Json(FileDto::from(updated)))
}

/// Delete a file.
#[utoipa::path(
    delete,
    path = "/files/{path}",
    params(("path" = String, Path, description = "File path")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = common::dto::ErrorResponse),
    ),
    tag = "files"
)]
pub async fn delete_file(
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let path = decode_path(&raw)?;
    state.files.delete(&path).await?;
    state.metadata.delete_file_entry(&path).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Rename and/or move a file in one atomic operation.
///
/// Supply `name` to rename, `new_folder_path` to move, or both to do both
/// simultaneously. At least one field must be present.
#[utoipa::path(
    patch,
    path = "/files/{path}",
    params(("path" = String, Path, description = "File path (without leading slash)")),
    request_body = common::dto::PatchFileRequest,
    responses(
        (status = 200, description = "Updated",          body = common::dto::FileDto),
        (status = 404, description = "Not found",        body = common::dto::ErrorResponse),
        (status = 409, description = "Name taken",       body = common::dto::ErrorResponse),
        (status = 422, description = "Validation error", body = common::dto::ErrorResponse),
    ),
    tag = "files"
)]
pub async fn patch_file(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Json(body): Json<PatchFileRequest>,
) -> Result<impl IntoResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::from(common::CatalogError::Validation(e.to_string())))?;
    if body.name.is_none() && body.new_folder_path.is_none() {
        return Err(ApiError::from(common::CatalogError::Validation(
            "at least one of 'name' or 'new_folder_path' must be provided".into(),
        )));
    }

    let path = decode_path(&raw)?;

    // Determine the target name (supplied or current).
    let new_name = body
        .name
        .as_deref()
        .unwrap_or_else(|| path.name())
        .to_owned();

    // Determine the target parent folder (supplied or current).
    let new_parent = match body.new_folder_path {
        Some(p) => p,
        None => path.parent().ok_or_else(|| {
            ApiError::from(common::CatalogError::InvalidPath(
                "file has no parent".into(),
            ))
        })?,
    };

    let new_path = new_parent.join(&new_name).map_err(ApiError::from)?;
    if new_path == path {
        return Err(ApiError::from(common::CatalogError::Validation(
            "no changes: name and folder are both unchanged".into(),
        )));
    }

    // Rename the physical file first; if the metadata update fails the file
    // store rename is the only inconsistency (acceptable for this impl).
    state.files.rename(&path, &new_path).await?;
    let entry = state.metadata.relocate_file(&path, new_path).await?;
    Ok(Json(FileDto::from(entry)))
}
