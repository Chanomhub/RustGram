mod config;
mod crypto;
mod error;
mod handlers;
mod middleware;
mod models;
mod services;
mod worker;

use axum::{
    routing::{get, post, delete},
    Router,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer};
use tracing::{info, Level};
use tracing_subscriber;

use crate::{
    config::Config,
    handlers::{admin, health, image, job, upload, url_upload},
    middleware::rate_limit::RateLimitLayer,
    models::FileReference,
    services::telegram::TelegramService,
    worker::{run_upload_worker, UploadJob},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .init();

    // Load configuration
    let config = Arc::new(Config::from_env()?);
    info!("Configuration loaded successfully");

    // Initialize services
    let telegram_service = Arc::new(TelegramService::new(
        config.telegram_bot_token.clone(),
        config.telegram_chat_id,
        None, // Consider adding a log_chat_id from config
    ));

    // Create a channel for the upload queue
    let (tx, rx) = mpsc::channel::<UploadJob>(100); // Buffer size of 100

    // Create a job store to hold job results
    let job_store = Arc::new(Mutex::new(HashMap::<String, FileReference>::new()));

    // Spawn the upload worker
    tokio::spawn(run_upload_worker(
        rx,
        job_store.clone(),
        telegram_service.clone(),
        config.clone(),
    ));

    // Build application state
    let app_state = Arc::new(AppState {
        config: config.clone(),
        telegram_service,
        admin_secret: config.admin_secret.clone(),
        upload_queue: tx,
        job_store,
    });

    // Build router
    let app = Router::new()
        .route("/health", get(health::health_check))
        .route("/upload", post(upload::upload_image))
        .route("/upload_from_url", post(url_upload::upload_from_url))
        .route("/job/:id", get(job::get_job_status)) // New route for job status
        .route("/image/:id", get(image::get_image))
        .route("/info/:id", get(image::get_image_info))
        .route("/admin/image/:id", delete(admin::delete_image))
        .layer(
            ServiceBuilder::new()
                .layer(RequestBodyLimitLayer::new(config.max_file_size))
                .layer(RateLimitLayer::new(config.rate_limit_per_minute))
                .layer(CorsLayer::permissive()),
        )
        .with_state(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
    info!("Server starting on {}", config.bind_address);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub telegram_service: Arc<TelegramService>,
    pub admin_secret: String,
    pub upload_queue: mpsc::Sender<UploadJob>,
    pub job_store: Arc<Mutex<HashMap<String, FileReference>>>,
}
