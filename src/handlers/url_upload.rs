use axum::{
    extract::{State, ConnectInfo},
    http::StatusCode,
    response::Json,
};
use std::net::SocketAddr;
use std::sync::Arc;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    models::QueuedResponse,
    worker::UploadJob,
    AppState,
};

#[derive(Deserialize)]
pub struct UrlUploadPayload {
    pub url: String,
}

pub async fn upload_from_url(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UrlUploadPayload>,
) -> Result<(StatusCode, Json<QueuedResponse>)> {
    // Download image from URL
    let response = reqwest::get(&payload.url).await.map_err(|e| {
        AppError::ValidationError(format!("Failed to download image from URL: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(AppError::ValidationError(format!(
            "Failed to download image: status code {}",
            response.status()
        )));
    }

    let image_data = response.bytes().await.map_err(|e| {
        AppError::ValidationError(format!("Failed to read image bytes: {}", e))
    })?.to_vec();
    let original_size = image_data.len();

    // --- All validations from here ---
    if original_size > state.config.max_file_size {
        return Err(AppError::FileTooLarge { max_size: state.config.max_file_size });
    }

    let mime_type = mime_guess::from_ext(payload.url.split('.').last().unwrap_or(""))
        .first_or_octet_stream();
    let final_mime_type = mime_type.to_string();

    if !state.config.allowed_image_types.contains(&final_mime_type) {
        return Err(AppError::InvalidFileFormat(format!(
            "Unsupported type: {}. Allowed: {:?}",
            final_mime_type, state.config.allowed_image_types
        )));
    }

    if let Err(e) = image::load_from_memory(&image_data) {
        return Err(AppError::InvalidFileFormat(format!("Invalid image data: {}", e)));
    }
    // --- End of validations ---

    // Encrypt image data
    let encryption_key = state.config.get_encryption_key_bytes()?;
    let crypto = CryptoService::new(&encryption_key);
    let encrypted_data = crypto.encrypt_data(&image_data)?;

    // Generate a unique job ID
    let job_id = Uuid::new_v4().to_string();

    // Generate unique filename for Telegram
    let filename = payload.url.split('/').last().unwrap_or("image.bin").to_string();
    let unique_filename = format!("{}_{}", Uuid::new_v4(), filename);

    // Create an upload job
    let job = UploadJob {
        job_id: job_id.clone(),
        encrypted_data,
        unique_filename,
        original_size,
        mime_type: final_mime_type.clone(),
        client_ip: addr,
    };

    // Send the job to the worker queue
    state.upload_queue.send(job).await.map_err(|e| {
        tracing::error!("Failed to send job to queue: {}", e);
        AppError::InternalError("Failed to queue upload job".to_string())
    })?;

    tracing::info!(
        "Queued job ID: {} for URL: {} and IP: {}. Size: {}, Type: {}",
        job_id, payload.url, addr, original_size, final_mime_type
    );

    // Respond to the client immediately
    let response = QueuedResponse {
        job_id: job_id.clone(),
        status_url: format!("/job/{}", job_id),
    };

    Ok((StatusCode::ACCEPTED, Json(response)))
}