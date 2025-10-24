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
    env,
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    port: u16,
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
            port: 3000,
            qbittorrent_host: "0.0.0.0".to_string(),
            qbittorrent_port: 8080,
            movies_directory: PathBuf::from("/media/movies"),
            series_directory: PathBuf::from("/media/series"),
        }
    }
}

impl AppConfig {
    fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(port) = env::var("APP_PORT") {
            config.port = port.parse().unwrap_or(config.port);
        }

        if let Ok(host) = env::var("QBITTORRENT_HOST") {
            config.qbittorrent_host = host;
        }
        if let Ok(port) = env::var("QBITTORRENT_PORT") {
            config.qbittorrent_port = port.parse().unwrap_or(config.qbittorrent_port);
        }
        if let Ok(movies_dir) = env::var("MOVIES_DIRECTORY") {
            config.movies_directory = PathBuf::from(movies_dir);
        }
        if let Ok(series_dir) = env::var("SERIES_DIRECTORY") {
            config.series_directory = PathBuf::from(series_dir);
        }

        config
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

    info!("Starting server with {:#?}", state.config);
    let port = state.config.port;

    let files = ServeDir::new("./wwwroot");
    let app = Router::new()
        .route("/api/qbittorrent/add", post(add_torrent))
        .fallback_service(files)
        .with_state(state);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    info!(
        "Server listening on http://{}",
        listener.local_addr().unwrap()
    );
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
