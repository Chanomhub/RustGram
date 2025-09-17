use axum::{
    extract::{State, ConnectInfo},
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;
use std::net::SocketAddr;
use serde::Deserialize;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    models::{FileReference, UploadResponse},
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
) -> Result<Json<UploadResponse>> {
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

    // Validate file size
    if image_data.len() > state.config.max_file_size {
        return Err(AppError::FileTooLarge {
            max_size: state.config.max_file_size,
        });
    }

    // Detect MIME type
    let mime_type = mime_guess::from_ext(
        payload.url.split('.').last().unwrap_or(""),
    )
    .first_or_octet_stream();
    let final_mime_type = mime_type.to_string();

    // Validate image type
    if !state.config.allowed_image_types.contains(&final_mime_type) {
        return Err(AppError::InvalidFileFormat(format!(
            "Unsupported image type: {}. Allowed types: {:?}",
            final_mime_type, state.config.allowed_image_types
        )));
    }

    // Validate image data by trying to decode it
    let _img = image::load_from_memory(&image_data)
        .map_err(|e| AppError::InvalidFileFormat(format!("Invalid image data: {}", e)))?;

    // Initialize crypto service
    let encryption_key = state.config.get_encryption_key_bytes()
        .map_err(|e| AppError::ConfigError(e.to_string()))?;
    let crypto = CryptoService::new(&encryption_key);

    // Encrypt image data
    let encrypted_data = crypto.encrypt_data(&image_data)?;

    // Generate unique filename for Telegram
    let filename = payload.url.split('/').last().unwrap_or("image.bin").to_string();
    let unique_filename = format!("{}_{}", Uuid::new_v4(), filename);

    // Upload to Telegram
    let telegram_message = state
        .telegram_service
        .upload_file(&encrypted_data, &unique_filename)
        .await?;

    // Extract file information
    let file_id = telegram_message
        .document
        .as_ref()
        .map(|doc| doc.file_id.clone())
        .ok_or_else(|| AppError::TelegramError("No document in response".to_string()))?;

    // Create file reference
    let file_ref = FileReference::new(
        file_id,
        telegram_message.message_id,
        image_data.len(),
        final_mime_type.clone(),
    );

    // Encrypt file reference for URL
    let encrypted_id = crypto.encrypt_file_reference(&file_ref)?;

    // Create response
    let response = UploadResponse {
        id: encrypted_id.clone(),
        url: format!("/image/{}", encrypted_id),
        size: image_data.len(),
        mime_type: final_mime_type.clone(),
    };

    tracing::info!(
        "Image uploaded successfully from URL: {}, size: {} bytes, type: {}",
        payload.url,
        image_data.len(),
        final_mime_type
    );

    state.telegram_service.send_log_message(&format!(
        "Image uploaded from URL: URL={}, ID={}, Size={}, Type={}, IP={}",
        payload.url,
        response.id,
        response.size,
        response.mime_type,
        addr
    )).await?;

    Ok(Json(response))
}
