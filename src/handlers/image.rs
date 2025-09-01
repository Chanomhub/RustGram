use axum::{
    extract::{Path, State, ConnectInfo},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use std::net::SocketAddr;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    AppState,
};

pub async fn get_image(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(encrypted_id): Path<String>,
) -> Result<Response> {
    // Initialize crypto service
    let encryption_key = state.config.get_encryption_key_bytes()
        .map_err(|e| AppError::ConfigError(e.to_string()))?;
    let crypto = CryptoService::new(&encryption_key);

    // Decrypt file reference
    let file_ref = crypto.decrypt_file_reference(&encrypted_id)?;

    // Download encrypted file from Telegram
    let encrypted_data = state
        .telegram_service
        .download_file_by_id(&file_ref.file_id)
        .await?;

    // Decrypt image data
    let image_data = crypto.decrypt_data(&encrypted_data)?;

    // Validate decrypted data size matches expected size
    if image_data.len() != file_ref.file_size {
        return Err(AppError::InternalError(
            "Decrypted file size mismatch".to_string(),
        ));
    }

    // Create response headers
    let mut headers = HeaderMap::new();
    
    // Set content type
    headers.insert(
        header::CONTENT_TYPE,
        file_ref.mime_type.parse()
            .map_err(|_| AppError::InternalError("Invalid MIME type".to_string()))?,
    );

    // Set content length
    headers.insert(
        header::CONTENT_LENGTH,
        image_data.len().to_string().parse()
            .map_err(|_| AppError::InternalError("Invalid content length".to_string()))?,
    );

    // Set cache headers (optional - cache for 1 hour)
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=3600".parse()
            .map_err(|_| AppError::InternalError("Invalid cache control".to_string()))?,
    );

    // Add ETag for caching
    let etag = format!("\"{}\"", hex::encode(&crate::crypto::CryptoService::hash_data(&image_data)[..8]));
    headers.insert(
        header::ETAG,
        etag.parse()
            .map_err(|_| AppError::InternalError("Invalid ETag".to_string()))?,
    );

    tracing::info!(
        "Image served successfully: {} bytes, type: {}",
        image_data.len(),
        file_ref.mime_type
    );

    state.telegram_service.send_log_message(&format!(
        "Image retrieved: ID={}, Size={}, Type={}, IP={}",
        encrypted_id,
        image_data.len(),
        file_ref.mime_type,
        addr
    )).await?;

    // Return image data with headers
    Ok((StatusCode::OK, headers, image_data).into_response())
}

// Alternative endpoint for getting image metadata without downloading
pub async fn get_image_info(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(encrypted_id): Path<String>,
) -> Result<axum::Json<serde_json::Value>> {
    // Initialize crypto service
    let encryption_key = state.config.get_encryption_key_bytes()
        .map_err(|e| AppError::ConfigError(e.to_string()))?;
    let crypto = CryptoService::new(&encryption_key);

    // Decrypt file reference
    let file_ref = crypto.decrypt_file_reference(&encrypted_id)?;

    let response = serde_json::json!({
        "size": file_ref.file_size,
        "mime_type": file_ref.mime_type,
        "id": encrypted_id
    });

    state.telegram_service.send_log_message(&format!(
        "Image info retrieved: ID={}, Size={}, Type={}, IP={}",
        encrypted_id,
        file_ref.file_size,
        file_ref.mime_type,
        addr
    )).await?;

    Ok(axum::Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{crypto::CryptoService, models::FileReference};

    #[tokio::test]
    async fn test_decrypt_file_reference() {
        let key = CryptoService::generate_key();
        let crypto = CryptoService::new(&key);
        
        let file_ref = FileReference::new(
            "test_file_id".to_string(),
            12345,
            1024,
            "image/jpeg".to_string(),
        );
        
        let encrypted_id = crypto.encrypt_file_reference(&file_ref).unwrap();
        let decrypted = crypto.decrypt_file_reference(&encrypted_id).unwrap();
        
        assert_eq!(file_ref.file_id, decrypted.file_id);
        assert_eq!(file_ref.message_id, decrypted.message_id);
    }
}
