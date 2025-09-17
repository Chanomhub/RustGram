use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Telegram API error: {0}")]
    TelegramError(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Invalid file format: {0}")]
    InvalidFileFormat(String),
    
    #[error("File too large: {max_size} bytes maximum")]
    FileTooLarge { max_size: usize },
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Image not found")]
    NotFound,
    
    #[error("Invalid image ID")]
    InvalidImageId,
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Invalid ID format")]
    InvalidId,

    #[error("Too many requests")]
    TooManyRequests,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::TelegramError(msg) => {
                tracing::error!("Telegram error: {}", msg);
                (StatusCode::SERVICE_UNAVAILABLE, "External service error".to_string())
            }
            AppError::EncryptionError(msg) => {
                tracing::error!("Encryption error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Encryption error".to_string())
            }
            AppError::InvalidFileFormat(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::FileTooLarge { max_size } => {
                (StatusCode::PAYLOAD_TOO_LARGE, 
                 format!("File too large. Maximum size: {} bytes", max_size))
            }
            AppError::RateLimitExceeded => {
                (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string())
            }
            AppError::TooManyRequests => {
                (StatusCode::TOO_MANY_REQUESTS, "Too many concurrent requests".to_string())
            }
            AppError::NotFound => {
                (StatusCode::NOT_FOUND, "Image not found".to_string())
            }
            AppError::InvalidImageId => {
                (StatusCode::BAD_REQUEST, "Invalid image ID".to_string())
            }
            AppError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::ValidationError(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::ConfigError(msg) => {
                tracing::error!("Configuration error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error".to_string())
            }
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            }
            AppError::InvalidId => {
                (StatusCode::BAD_REQUEST, "Invalid ID format".to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

// Convenience type alias
pub type Result<T> = std::result::Result<T, AppError>;

// Convert from other error types
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::TelegramError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InternalError(format!("JSON error: {}", err))
    }
}

impl From<base64::DecodeError> for AppError {
    fn from(_err: base64::DecodeError) -> Self {
        AppError::InvalidImageId
    }
}

impl From<image::ImageError> for AppError {
    fn from(err: image::ImageError) -> Self {
        AppError::InvalidFileFormat(format!("Image processing error: {}", err))
    }
}