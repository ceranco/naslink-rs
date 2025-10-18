use std::{path::PathBuf, sync::Arc};

use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    qbittorrent_host: String,
    qbittorrent_port: u16,
    movies_directory: PathBuf,
    series_directory: PathBuf,
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
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        config: AppConfig::from_env(),
    });

    let app = Router::new()
        .route("/json", get(json))
        .route("/plaintext", get(plain_text))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn json(State(state): State<Arc<AppState>>) -> Json<AppConfig> {
    Json(state.config.clone())
}

async fn plain_text() -> &'static str {
    "Hello, World!"
}
