use axum::{
    Router,
    routing::{get, patch},
};
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
        folders::delete_folder,
        folders::rename_folder,
        folders::move_folder,
        files::upload_file,
        files::download_file,
        files::reupload_file,
        files::delete_file,
        files::rename_file,
        files::move_file,
    ),
    components(schemas(
        common::dto::FolderDto,
        common::dto::FolderContentsDto,
        common::dto::FileDto,
        common::dto::CreateFolderRequest,
        common::dto::RenameFolderRequest,
        common::dto::MoveFolderRequest,
        common::dto::RenameFileRequest,
        common::dto::MoveFileRequest,
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

    // NOTE: axum uses `matchit` which requires wildcard segments (`{*path}`)
    // to be terminal — they cannot be followed by any static suffix.
    // Rename and move operations therefore use an action-first URL prefix so
    // that the wildcard remains at the end of the pattern.
    //
    // `.with_state()` erases the state parameter (Router<AppState> → Router<()>),
    // which is required before merging SwaggerUi (Into<Router<()>> only).
    // Type annotation resolves the S2 ambiguity in Router::with_state<S2>.
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
                .delete(folders::delete_folder),
        )
        .route("/folder-rename/{*path}", patch(folders::rename_folder))
        .route("/folder-move/{*path}", patch(folders::move_folder))
        // ── Files ─────────────────────────────────────────────────────────
        .route(
            "/files/{*path}",
            get(files::download_file)
                .post(files::upload_file)
                .put(files::reupload_file)
                .delete(files::delete_file),
        )
        .route("/file-rename/{*path}", patch(files::rename_file))
        .route("/file-move/{*path}", patch(files::move_file))
        .with_state(state);

    // Swagger UI only implements Into<Router<()>>, so merge after with_state.
    api_router.merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", api_doc))
}
