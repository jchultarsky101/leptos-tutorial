use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

pub mod files;
pub mod folders;

#[derive(OpenApi)]
#[openapi(
    paths(
        folders::list_root,
        folders::list_folder,
        folders::create_folder_in_root,
        folders::create_folder,
        folders::patch_folder,
        folders::delete_folder,
        files::upload_file,
        files::download_file,
        files::reupload_file,
        files::patch_file,
        files::delete_file,
    ),
    components(schemas(
        common::dto::FolderDto,
        common::dto::FolderContentsDto,
        common::dto::FileDto,
        common::dto::CreateFolderRequest,
        common::dto::PatchFolderRequest,
        common::dto::PatchFileRequest,
        common::dto::ErrorResponse,
        common::path::CatalogPath,
    )),
    tags(
        (name = "folders", description = "Virtual folder operations"),
        (name = "files",   description = "File upload, download, and management"),
    ),
    info(
        title = "File Catalog API",
        version = "0.1.0",
        description = "REST API for a virtual file catalog with folder hierarchies.",
    )
)]
pub struct ApiDoc;

pub fn build_router(state: AppState) -> Router {
    let api_doc = ApiDoc::openapi();

    // Both PATCH handlers are mounted on the same wildcard routes as the other
    // methods — PATCH /folders/{*path} and PATCH /files/{*path} — which is
    // idiomatic REST.  The `{*path}` wildcard must remain terminal in matchit,
    // so all operations on a resource are grouped on a single route pattern.
    //
    // `.with_state()` erases the state parameter (Router<AppState> →
    // Router<()>), required before merging SwaggerUi (Into<Router<()>> only).
    // The explicit type annotation resolves the S2 ambiguity in with_state<S2>.
    let api_router: Router = Router::new()
        // ── Folders ──────────────────────────────────────────────────────
        .route(
            "/folders",
            get(folders::list_root).post(folders::create_folder_in_root),
        )
        .route(
            "/folders/{*path}",
            get(folders::list_folder)
                .post(folders::create_folder)
                .patch(folders::patch_folder)
                .delete(folders::delete_folder),
        )
        // ── Files ─────────────────────────────────────────────────────────
        .route(
            "/files/{*path}",
            get(files::download_file)
                .post(files::upload_file)
                .put(files::reupload_file)
                .patch(files::patch_file)
                .delete(files::delete_file),
        )
        .with_state(state);

    // Swagger UI only implements Into<Router<()>>, so merge after with_state.
    // CorsLayer::permissive() allows the Leptos WASM frontend (localhost:8080)
    // to reach this API (localhost:3000) during local development.
    api_router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", api_doc))
        .layer(CorsLayer::permissive())
}
