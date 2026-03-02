use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use common::{CatalogError, dto::ErrorResponse};

/// Wrapper that converts a [`CatalogError`] into an Axum HTTP response.
///
/// Handlers return `Result<T, ApiError>`. The `?` operator converts
/// `CatalogError` into `ApiError` via the `From` impl below.
pub struct ApiError(CatalogError);

impl From<CatalogError> for ApiError {
    fn from(e: CatalogError) -> Self {
        ApiError(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, detail) = match &self.0 {
            CatalogError::FolderNotFound(_) | CatalogError::FileNotFound(_) => {
                (StatusCode::NOT_FOUND, None)
            }
            CatalogError::AlreadyExists(_) => (StatusCode::CONFLICT, None),
            CatalogError::InvalidPath(_)
            | CatalogError::Validation(_)
            | CatalogError::CircularMove => (StatusCode::UNPROCESSABLE_ENTITY, None),
            CatalogError::CannotDeleteRoot => (StatusCode::FORBIDDEN, None),
            CatalogError::Storage(_) | CatalogError::Io(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("internal storage error"),
            ),
        };

        let body = ErrorResponse {
            error: self.0.to_string(),
            detail: detail.map(str::to_owned),
        };

        (status, Json(body)).into_response()
    }
}
