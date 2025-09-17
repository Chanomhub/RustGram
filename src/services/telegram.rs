use bytes::Bytes;
use reqwest::{multipart, Client};
use crate::{
    error::{AppError, Result},
    models::{TelegramFile, TelegramMessage, TelegramResponse},
};

pub struct TelegramService {
    client: Client,
    bot_token: String,
    chat_id: i64,
    log_chat_id: Option<i64>, // New field for logging
    base_url: String,
}

impl TelegramService {
    pub fn new(bot_token: String, chat_id: i64, log_chat_id: Option<i64>) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("https://api.telegram.org/bot{}", bot_token),
            bot_token,
            chat_id,
            log_chat_id, // Initialize new field
        }
    }

    /// Upload file to Telegram and return file info
    pub async fn upload_file(&self, data: &[u8], filename: &str) -> Result<TelegramMessage> {
        let form = multipart::Form::new()
            .text("chat_id", self.chat_id.to_string())
            .part(
                "document",
                multipart::Part::bytes(data.to_vec())
                    .file_name(filename.to_string())
                    .mime_str("application/octet-stream")
                    .map_err(|e| AppError::InternalError(e.to_string()))?,
            );

        let url = format!("{}/sendDocument", self.base_url);
        
        let response = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TelegramError(format!(
                "Upload failed: {}",
                error_text
            )));
        }

        let telegram_response: TelegramResponse<TelegramMessage> = response.json().await?;

        if !telegram_response.ok {
            return Err(AppError::TelegramError(
                telegram_response.description.unwrap_or_default(),
            ));
        }

        telegram_response
            .result
            .ok_or_else(|| AppError::TelegramError("No result in response".to_string()))
    }

    /// Get file info from Telegram
    pub async fn get_file_info(&self, file_id: &str) -> Result<TelegramFile> {
        let url = format!("{}/getFile", self.base_url);
        
        let response = self
            .client
            .post(&url)
            .form(&[("file_id", file_id)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::TelegramError("Failed to get file info".to_string()));
        }

        let telegram_response: TelegramResponse<TelegramFile> = response.json().await?;

        if !telegram_response.ok {
            return Err(AppError::TelegramError(
                telegram_response.description.unwrap_or_default(),
            ));
        }

        telegram_response
            .result
            .ok_or_else(|| AppError::TelegramError("No file info in response".to_string()))
    }

    /// Download file from Telegram
    pub async fn download_file(&self, file_path: &str) -> Result<Bytes> {
        let download_url = format!("https://api.telegram.org/file/bot{}/{}", 
                                 self.bot_token, file_path);
        
        let response = self
            .client
            .get(&download_url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::TelegramError("Failed to download file".to_string()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes)
    }

    /// Download file by file_id (combines get_file_info and download_file)
    pub async fn download_file_by_id(&self, file_id: &str) -> Result<Bytes> {
        let file_info = self.get_file_info(file_id).await?;
        
        let file_path = file_info
            .file_path
            .ok_or_else(|| AppError::TelegramError("No file path in response".to_string()))?;

        self.download_file(&file_path).await
    }

    /// Delete message (to clean up if needed)
    pub async fn delete_message(&self, chat_id: i64, message_id: i64) -> Result<()> {
        let url = format!("{}/deleteMessage", self.base_url);
        
        let response = self
            .client
            .post(&url)
            .form(&[
                ("chat_id", chat_id.to_string()),
                ("message_id", message_id.to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TelegramError(format!(
                "Failed to delete message: {}",
                error_text
            )));
        }

        let telegram_response: TelegramResponse<bool> = response.json().await?;

        if !telegram_response.ok {
            return Err(AppError::TelegramError(
                telegram_response.description.unwrap_or_default(),
            ));
        }

        Ok(())
    }

    /// Send a log message to the configured log chat ID
    pub async fn send_log_message(&self, message: &str) -> Result<()> {
        if let Some(log_chat_id) = self.log_chat_id {
            let url = format!("{}/sendMessage", self.base_url);
            let response = self
                .client
                .post(&url)
                .form(&[
                    ("chat_id", log_chat_id.to_string()),
                    ("text", message.to_string()),
                ])
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(AppError::TelegramError(format!(
                    "Failed to send log message: {}",
                    error_text
                )));
            }

            let telegram_response: TelegramResponse<TelegramMessage> = response.json().await?;
            if !telegram_response.ok {
                return Err(AppError::TelegramError(
                    telegram_response.description.unwrap_or_default(),
                ));
            }
        }
        Ok(())
    }

    /// Test bot connection
    pub async fn test_connection(&self) -> Result<()> {
        let url = format!("{}/getMe", self.base_url);
        
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AppError::TelegramError("Bot connection test failed".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telegram_service_creation() {
        let service = TelegramService::new("test_token".to_string(), 12345);
        assert_eq!(service.chat_id, 12345);
        assert!(service.base_url.contains("test_token"));
    }
}
