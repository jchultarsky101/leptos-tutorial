use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use storage::{filesystem::LocalFileStore, memory::InMemoryMetadataStore};

mod error;
mod routes;
mod state;
mod storage;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let storage_dir = std::env::var("CATALOG_STORAGE_DIR").unwrap_or_else(|_| "./data".to_owned());

    let file_store = LocalFileStore::new(&storage_dir).await.unwrap_or_else(|e| {
        tracing::error!("failed to initialise file store at {storage_dir:?}: {e}");
        std::process::exit(1);
    });

    let metadata_store = InMemoryMetadataStore::new();

    // Scan the storage directory to re-seed metadata from files on disk.
    match metadata_store.scan_directory(Path::new(&storage_dir)).await {
        Ok((dirs, files)) => {
            tracing::info!("scanned {storage_dir:?}: {dirs} folders, {files} files");
        }
        Err(e) => {
            tracing::warn!("failed to scan {storage_dir:?}: {e}");
        }
    }

    let state = AppState {
        metadata: Arc::new(metadata_store),
        files: Arc::new(file_store),
    };

    let router = routes::build_router(state);

    let port: u16 = std::env::var("CATALOG_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    tracing::info!("Swagger UI available at http://{addr}/swagger-ui/");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("failed to bind {addr}: {e}");
            std::process::exit(1);
        });

    axum::serve(listener, router).await.unwrap_or_else(|e| {
        tracing::error!("server error: {e}");
        std::process::exit(1);
    });
}
