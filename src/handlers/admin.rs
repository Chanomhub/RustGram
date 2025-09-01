use axum::{
    extract::{Path, State, ConnectInfo},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::info;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::{
    error::AppError,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct AdminDeleteRequest {
    api_key: String,
}

pub async fn delete_image(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<AdminDeleteRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Basic API Key authentication
    if payload.api_key != state.admin_secret {
        info!("Unauthorized attempt to delete image: {} from IP: {}", id, addr);
        state.telegram_service.send_log_message(&format!("Unauthorized delete attempt for image ID: {} from IP: {}", id, addr)).await?;
        return Err(AppError::Unauthorized);
    }

    info!("Attempting to delete image with ID: {} from IP: {}", id, addr);

    // Extract chat_id and message_id from the image ID
    let parts: Vec<&str> = id.split('_').collect();
    if parts.len() != 2 {
        info!("Invalid image ID format for deletion: {} from IP: {}", id, addr);
        state.telegram_service.send_log_message(&format!("Invalid image ID format for deletion: {} from IP: {}", id, addr)).await?;
        return Err(AppError::InvalidId);
    }

    let chat_id = parts[0].parse::<i64>().map_err(|_| AppError::InvalidId)?;
    let message_id = parts[1].parse::<i64>().map_err(|_| AppError::InvalidId)?;

    match state.telegram_service.delete_message(chat_id, message_id).await {
        Ok(_) => {
            info!("Successfully deleted image with ID: {} from IP: {}", id, addr);
            state.telegram_service.send_log_message(&format!("Image deleted: {} by IP: {}", id, addr)).await?;
            Ok(StatusCode::OK)
        }
        Err(e) => {
            info!("Failed to delete image with ID {}: {:?} from IP: {}", id, e, addr);
            state.telegram_service.send_log_message(&format!("Failed to delete image {}: {:?} by IP: {}", id, e, addr)).await?;
            Err(e)
        }
    }
}
