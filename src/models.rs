use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReference {
    pub file_id: String,
    pub message_id: i64,
    pub nonce: [u8; 12], // AES-GCM nonce
    pub size: usize,
    pub mime_type: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub url: String,
    pub size: usize,
    pub mime_type: String,
}

// The immediate response when a file is queued for upload
#[derive(Debug, Serialize)]
pub struct QueuedResponse {
    pub job_id: String,
    pub status_url: String,
}

// Represents the status of an upload job
#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum JobStatus {
    Pending,
    Completed { response: UploadResponse },
    Failed { error: String },
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramFile {
    pub file_id: String,
    pub file_unique_id: String,
    pub file_size: Option<i64>,
    pub file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub document: Option<TelegramDocument>,
    pub photo: Option<Vec<TelegramPhotoSize>>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramDocument {
    pub file_id: String,
    pub file_unique_id: String,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramPhotoSize {
    pub file_id: String,
    pub file_unique_id: String,
    pub width: i32,
    pub height: i32,
    pub file_size: Option<i64>,
}

impl FileReference {
    pub fn new(
        file_id: String,
        message_id: i64,
        size: usize,
        mime_type: String,
    ) -> Self {
        let mut nonce = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
        
        Self {
            file_id,
            message_id,
            nonce,
            size,
            mime_type,
        }
    }
} 
