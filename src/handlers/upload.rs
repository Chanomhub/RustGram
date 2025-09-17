use axum::{
    extract::{Multipart, State, ConnectInfo},
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;
use std::net::SocketAddr;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    models::{FileReference, UploadResponse},
    AppState,
};

pub async fn upload_image(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>> {
    let mut image_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut mime_type: Option<String> = None;

    // Process multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::ValidationError(format!("Invalid multipart data: {}", e)))?
    {
        match field.name() {
            Some("image") | Some("file") => {
                // Get content type
                if let Some(content_type) = field.content_type() {
                    mime_type = Some(content_type.to_string());
                }

                // Get filename
                if let Some(field_filename) = field.file_name() {
                    filename = Some(field_filename.to_string());
                }

                // Read file data
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::ValidationError(format!("Failed to read file: {}", e)))?;

                image_data = Some(data.to_vec());
            }
            _ => {
                // Skip unknown fields
                continue;
            }
        }
    }

    let image_data = image_data.ok_or_else(|| {
        AppError::ValidationError("No image file found in request".to_string())
    })?;

    // Validate file size
    if image_data.len() > state.config.max_file_size {
        return Err(AppError::FileTooLarge {
            max_size: state.config.max_file_size,
        });
    }

    // Detect MIME type if not provided
    let detected_mime = mime_guess::from_path(
        filename.as_deref().unwrap_or("unknown")
    ).first_or_octet_stream();
    
    let final_mime_type = mime_type.unwrap_or_else(|| detected_mime.to_string());

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
    let unique_filename = format!("{}_{}", 
        Uuid::new_v4(), 
        filename.unwrap_or_else(|| "image.bin".to_string())
    );

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
        "Image uploaded successfully: {} bytes, type: {}",
        image_data.len(),
        final_mime_type
    );

    state.telegram_service.send_log_message(&format!(
        "Image uploaded: ID={}, Size={}, Type={}, IP={}",
        response.id,
        response.size,
        response.mime_type,
        addr
    )).await?;

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, header::CONTENT_TYPE},
    };

    #[tokio::test]
    async fn test_upload_validation() {
        // Test that we properly validate file size and type
        // This would require setting up a test server
        // For now, this is a placeholder for future tests
    }
}