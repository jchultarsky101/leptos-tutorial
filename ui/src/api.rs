use common::{
    CatalogPath,
    dto::{
        FileDto, FolderContentsDto, FolderDto, PatchFileRequest, PatchFolderRequest,
        SearchResultsDto,
    },
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

/// Fetch the raw text content of a file for preview purposes.
///
/// Returns `Err(UiError::FileTooLarge)` without reading the body when the
/// server reports a `Content-Length` larger than 1 MiB.
pub async fn fetch_file_content(path: &CatalogPath) -> Result<String, UiError> {
    const MAX_BYTES: u64 = 1_048_576; // 1 MiB

    let url = format!("{API_BASE}/files/{}", path_segment(path));
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;

    if !resp.ok() {
        return Err(api_error(resp).await);
    }

    // Check Content-Length before touching the body — avoids pulling a huge
    // file over the wire before we can show the "too large" message.
    if let Some(cl) = resp.headers().get("content-length")
        && let Ok(n) = cl.parse::<u64>()
        && n > MAX_BYTES
    {
        return Err(UiError::FileTooLarge(n));
    }

    resp.text().await.map_err(|e| UiError::Parse(e.to_string()))
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

// ── Search ───────────────────────────────────────────────────────────────────

/// Search files and folders by name and/or content.
pub async fn search(query: String, fuzzy: bool) -> Result<SearchResultsDto, UiError> {
    let encoded = js_sys::encode_uri_component(&query);
    let url = format!("{API_BASE}/search?q={encoded}&fuzzy={fuzzy}");
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;
    if resp.ok() {
        resp.json::<SearchResultsDto>()
            .await
            .map_err(|e| UiError::Parse(e.to_string()))
    } else {
        Err(api_error(resp).await)
    }
}

// ── Statistics ────────────────────────────────────────────────────────────────

/// Fetch aggregate catalog statistics.
pub async fn get_stats() -> Result<common::dto::StatsDto, UiError> {
    let url = format!("{API_BASE}/stats");
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| UiError::Network(e.to_string()))?;
    if resp.ok() {
        resp.json::<common::dto::StatsDto>()
            .await
            .map_err(|e| UiError::Parse(e.to_string()))
    } else {
        Err(api_error(resp).await)
    }
}

/// Replace the text content of an existing file.
///
/// Wraps `content` in a `Blob` (MIME type `text/markdown`) and sends it to
/// `PUT /files/{path}` as multipart/form-data, which overwrites the file in
/// place while preserving its metadata entry.
pub async fn save_file_text(path: &CatalogPath, content: &str) -> Result<FileDto, UiError> {
    use js_sys::Array;
    use wasm_bindgen::JsValue;

    let arr = Array::new();
    arr.push(&JsValue::from_str(content));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("text/markdown");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&arr, &opts).map_err(|e| {
        UiError::Network(
            e.as_string()
                .unwrap_or_else(|| "blob creation failed".into()),
        )
    })?;

    let form = web_sys::FormData::new().map_err(|e| {
        UiError::Network(
            e.as_string()
                .unwrap_or_else(|| "FormData creation failed".into()),
        )
    })?;
    form.append_with_blob_and_filename("file", &blob, path.name())
        .map_err(|e| {
            UiError::Network(
                e.as_string()
                    .unwrap_or_else(|| "FormData append failed".into()),
            )
        })?;

    let url = format!("{API_BASE}/files/{}", path_segment(path));
    let resp = Request::put(&url)
        .body(JsValue::from(form))
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
