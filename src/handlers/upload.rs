use axum::{
    extract::{Multipart, State, ConnectInfo},
    http::StatusCode,
    response::Json,
};
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    models::QueuedResponse,
    worker::UploadJob,
    AppState,
};

pub async fn upload_image(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<QueuedResponse>)> {
    let mut image_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut mime_type: Option<String> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await? {
        if let Some(name) = field.name() {
            if name == "image" || name == "file" {
                mime_type = field.content_type().map(|s| s.to_string());
                filename = field.file_name().map(|s| s.to_string());
                image_data = Some(field.bytes().await?.to_vec());
                break; // Found the image, no need to process further
            }
        }
    }

    let image_data = image_data.ok_or_else(|| AppError::ValidationError("No image found".into()))?;
    let original_size = image_data.len();

    // --- All validations from here ---
    if original_size > state.config.max_file_size {
        return Err(AppError::FileTooLarge { max_size: state.config.max_file_size });
    }

    let final_mime_type = mime_type.unwrap_or_else(|| {
        mime_guess::from_path(filename.as_deref().unwrap_or("")).first_or_octet_stream().to_string()
    });

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
    let unique_filename = format!(
        "{}_{}",
        Uuid::new_v4(),
        filename.unwrap_or_else(|| "image.bin".to_string())
    );

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
        "Queued job ID: {} for IP: {}. Size: {}, Type: {}",
        job_id, addr, original_size, final_mime_type
    );

    // Respond to the client immediately
    let response = QueuedResponse {
        job_id: job_id.clone(),
        status_url: format!("/job/{}", job_id),
    };

    Ok((StatusCode::ACCEPTED, Json(response)))
}
