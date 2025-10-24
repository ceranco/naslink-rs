use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
};
use log::{LevelFilter, error, info};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    qbittorrent_host: String,
    qbittorrent_port: u16,
    movies_directory: PathBuf,
    series_directory: PathBuf,
}

impl AppConfig {
    fn get_directory_path(&self, directory: Directory) -> &Path {
        match directory {
            Directory::Movies => &self.movies_directory,
            Directory::Series => &self.series_directory,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Directory {
    Movies,
    Series,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            qbittorrent_host: "0.0.0.0".to_string(),
            qbittorrent_port: 8080,
            movies_directory: PathBuf::from("/media/movies"),
            series_directory: PathBuf::from("/media/series"),
        }
    }
}

impl AppConfig {
    fn from_env() -> Self {
        // Placeholder for environment variable loading logic
        Self::default()
    }
}

#[derive(Debug, Clone)]
struct AppState {
    config: AppConfig,
    client: Client,
}

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .with_colors(true)
        .env()
        .init()
        .unwrap();

    let state = Arc::new(AppState {
        config: AppConfig::from_env(),
        client: Client::new(),
    });

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!(
        "Starting server on http://{} with {:?}",
        listener.local_addr().unwrap(),
        state.config
    );

    let files = ServeDir::new("./wwwroot");
    let app = Router::new()
        .route("/api/qbittorrent/add", post(add_torrent))
        .fallback_service(files)
        .with_state(state);

    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, Deserialize)]
struct AddTorrentRequest {
    url: String,
    directory: Directory,
}

async fn add_torrent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AddTorrentRequest>,
) -> Response {
    async fn add(state: Arc<AppState>, request: AddTorrentRequest) -> Result<(), Box<dyn Error>> {
        let api_endpoint = format!(
            "http://{}:{}/api/v2/torrents/add",
            state.config.qbittorrent_host, state.config.qbittorrent_port
        );
        let result = state
            .client
            .post(api_endpoint)
            .form(&[
                ("urls", request.url.as_str()),
                (
                    "savepath",
                    state
                        .config
                        .get_directory_path(request.directory)
                        .to_str()
                        .unwrap(),
                ),
            ])
            .send()
            .await?;
        result.error_for_status()?;

        Ok(())
    }

    info!("Adding torrent: {:?}", request);
    match add(state, request).await {
        Ok(_) => (StatusCode::OK, "Torrent added successfully").into_response(),
        Err(e) => {
            error!("Failed to add torrent: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to add torrent").into_response()
        }
    }
}
