use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

use crate::{
    config::Config,
    error::AppError,
    models::FileReference,
    services::telegram::TelegramService,
};

// The job that will be sent to the upload worker
#[derive(Debug)]
pub struct UploadJob {
    pub job_id: String,
    pub encrypted_data: Vec<u8>,
    pub unique_filename: String,
    pub original_size: usize,
    pub mime_type: String,
    pub client_ip: SocketAddr,
}

// The store for completed job results
pub type JobStore = Arc<Mutex<HashMap<String, FileReference>>>;

pub async fn run_upload_worker(
    mut rx: Receiver<UploadJob>,
    job_store: JobStore,
    telegram_service: Arc<TelegramService>,
    config: Arc<Config>,
) {
    tracing::info!("Upload worker started");

    while let Some(job) = rx.recv().await {
        tracing::info!("Processing job ID: {}", job.job_id);

        let result = process_job(&job, &telegram_service, &job_store).await;

        let log_message = match &result {
            Ok(_) => format!(
                "✅ Upload Success | Job ID: {} | Size: {} | Type: {} | IP: {}",
                job.job_id, job.original_size, job.mime_type, job.client_ip
            ),
            Err(e) => format!(
                "❌ Upload Failed | Job ID: {} | Error: {} | IP: {}",
                job.job_id, e, job.client_ip
            ),
        };
        
        if let Err(e) = telegram_service.send_log_message(&log_message).await {
            tracing::error!("Failed to send log message for job {}: {}", job.job_id, e);
        }

        if let Err(e) = result {
            tracing::error!("Failed to process job ID {}: {}", job.job_id, e);
            // In a real-world scenario, you might want to add the job to a dead-letter queue
            // or implement a retry mechanism with backoff.
        }

        // Apply a delay after each job processing to respect Telegram's rate limits
        tokio::time::sleep(Duration::from_secs(config.upload_delay_secs)).await;
    }

    tracing::info!("Upload worker shutting down");
}

async fn process_job(
    job: &UploadJob,
    telegram_service: &Arc<TelegramService>,
    job_store: &JobStore,
) -> Result<(), AppError> {
    // Upload to Telegram
    let telegram_message = telegram_service
        .upload_file(&job.encrypted_data, &job.unique_filename)
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
        job.original_size,
        job.mime_type.clone(),
    );

    // Store the result in the job store
    {
        let mut store = job_store.lock().map_err(|_| {
            AppError::InternalError("Failed to acquire job store lock".to_string())
        })?;
        store.insert(job.job_id.clone(), file_ref);
    }

    tracing::info!("Job ID {} processed and stored successfully", job.job_id);

    Ok(())
}