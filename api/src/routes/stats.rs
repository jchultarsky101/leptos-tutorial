use axum::{Json, extract::State, response::IntoResponse};
use common::dto::StatsDto;

use crate::{error::ApiError, state::AppState};

/// Retrieve aggregate statistics for the catalog.
#[utoipa::path(
    get,
    path = "/stats",
    responses(
        (status = 200, description = "Catalog statistics", body = StatsDto),
        (status = 500, description = "Internal server error"),
    ),
    tag = "stats",
)]
pub async fn get_stats(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let dto = state.metadata.stats().await?;
    Ok(Json(dto))
}
