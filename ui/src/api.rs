use common::{
    CatalogPath,
    dto::{FileDto, FolderContentsDto, FolderDto, PatchFileRequest, PatchFolderRequest},
};
use gloo_net::http::Request;

use crate::error::UiError;

const API_BASE: &str = "http://localhost:3000";

// ── Helper ────────────────────────────────────────────────────────────────────

/// Encode a `CatalogPath` as a URL segment (strips the leading `/`).
fn path_segment(p: &CatalogPath) -> &str {
    p.as_str().trim_start_matches('/')
}

/// Extract an error message from a non-2xx response.
async fn api_error(resp: gloo_net::http::Response) -> UiError {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    // Try to parse as ErrorResponse JSON for a nicer message.
    let message = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or(text);
    UiError::api(status, message)
}

// ── Folders ───────────────────────────────────────────────────────────────────

/// List the contents of a folder (or root `/`).
pub async fn list_folder(path: CatalogPath) -> Result<FolderContentsDto, UiError> {
    let url = if path.is_root() {
        format!("{API_BASE}/folders")
    } else {
        format!("{API_BASE}/folders/{}", path_segment(&path))
    };
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;
    if resp.ok() {
        resp.json::<FolderContentsDto>()
            .await
            .map_err(|e| UiError::Parse(e.to_string()))
    } else {
        Err(api_error(resp).await)
    }
}

/// Create a new subfolder under `parent`.
pub async fn create_folder(parent: CatalogPath, name: &str) -> Result<FolderDto, UiError> {
    let url = if parent.is_root() {
        format!("{API_BASE}/folders")
    } else {
        format!("{API_BASE}/folders/{}", path_segment(&parent))
    };
    let body = serde_json::json!({ "name": name });
    let resp = Request::post(&url)
        .json(&body)
        .map_err(|e| UiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;
    if resp.ok() {
        resp.json::<FolderDto>()
            .await
            .map_err(|e| UiError::Parse(e.to_string()))
    } else {
        Err(api_error(resp).await)
    }
}

/// Delete a folder recursively.
pub async fn delete_folder(path: CatalogPath) -> Result<(), UiError> {
    let url = format!("{API_BASE}/folders/{}", path_segment(&path));
    let resp = Request::delete(&url)
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;
    if resp.ok() {
        Ok(())
    } else {
        Err(api_error(resp).await)
    }
}

/// Rename and/or move a folder (at least one field must be `Some`).
pub async fn patch_folder(
    path: CatalogPath,
    req: PatchFolderRequest,
) -> Result<FolderDto, UiError> {
    let url = format!("{API_BASE}/folders/{}", path_segment(&path));
    let resp = Request::patch(&url)
        .json(&req)
        .map_err(|e| UiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;
    if resp.ok() {
        resp.json::<FolderDto>()
            .await
            .map_err(|e| UiError::Parse(e.to_string()))
    } else {
        Err(api_error(resp).await)
    }
}

// ── Files ─────────────────────────────────────────────────────────────────────

/// Upload a file into `folder` using multipart/form-data.
///
/// If `overwrite` is `true` the request uses PUT (replace content); otherwise
/// POST (create new). The browser sets the multipart boundary automatically
/// when a `FormData` body is used.
pub async fn upload_file(
    folder: &CatalogPath,
    name: &str,
    file: web_sys::File,
    overwrite: bool,
) -> Result<FileDto, UiError> {
    use wasm_bindgen::JsCast;
    let form = web_sys::FormData::new().map_err(|e| {
        UiError::Network(
            e.as_string()
                .unwrap_or_else(|| "FormData creation failed".into()),
        )
    })?;
    form.append_with_blob("file", file.unchecked_ref())
        .map_err(|e| {
            UiError::Network(
                e.as_string()
                    .unwrap_or_else(|| "FormData append failed".into()),
            )
        })?;

    let segment = if folder.is_root() {
        name.to_owned()
    } else {
        format!("{}/{name}", path_segment(folder))
    };
    let url = format!("{API_BASE}/files/{segment}");

    let resp = if overwrite {
        Request::put(&url)
    } else {
        Request::post(&url)
    }
    .body(wasm_bindgen::JsValue::from(form))
    .map_err(|e| UiError::Network(e.to_string()))?
    .send()
    .await
    .map_err(|e| UiError::Network(e.to_string()))?;

    if resp.ok() {
        resp.json::<FileDto>()
            .await
            .map_err(|e| UiError::Parse(e.to_string()))
    } else {
        Err(api_error(resp).await)
    }
}

/// Delete a file.
pub async fn delete_file(path: CatalogPath) -> Result<(), UiError> {
    let url = format!("{API_BASE}/files/{}", path_segment(&path));
    let resp = Request::delete(&url)
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;
    if resp.ok() {
        Ok(())
    } else {
        Err(api_error(resp).await)
    }
}

/// Rename and/or move a file (at least one field must be `Some`).
pub async fn patch_file(path: CatalogPath, req: PatchFileRequest) -> Result<FileDto, UiError> {
    let url = format!("{API_BASE}/files/{}", path_segment(&path));
    let resp = Request::patch(&url)
        .json(&req)
        .map_err(|e| UiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;
    if resp.ok() {
        resp.json::<FileDto>()
            .await
            .map_err(|e| UiError::Parse(e.to_string()))
    } else {
        Err(api_error(resp).await)
    }
}
