use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    models::{JobStatus, UploadResponse},
    AppState,
};

pub async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<(StatusCode, Json<JobStatus>)> {
    let job_store = state.job_store.lock().map_err(|_| {
        AppError::InternalError("Failed to acquire job store lock".to_string())
    })?;

    match job_store.get(&job_id) {
        Some(file_ref) => {
            // Job is complete, create the final response
            let encryption_key = state.config.get_encryption_key_bytes()?;
            let crypto = CryptoService::new(&encryption_key);
            let encrypted_id = crypto.encrypt_file_reference(file_ref)?;

            let response = UploadResponse {
                id: encrypted_id.clone(),
                url: format!("/image/{}", encrypted_id),
                size: file_ref.size,
                mime_type: file_ref.mime_type.clone(),
            };

            Ok((
                StatusCode::OK,
                Json(JobStatus::Completed { response }),
            ))
        }
        None => {
            // Job not found, which means it's pending or the ID is invalid
            Ok((StatusCode::ACCEPTED, Json(JobStatus::Pending)))
        }
    }
}
