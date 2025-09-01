use axum::{extract::State, http::StatusCode, response::Json};
use std::{sync::Arc, time::SystemTime};

use crate::{models::HealthResponse, AppState};

pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, StatusCode> {
    // Test Telegram connection
    match state.telegram_service.test_connection().await {
        Ok(_) => {
            let response = HealthResponse {
                status: "healthy".to_string(),
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}