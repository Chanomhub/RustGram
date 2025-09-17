use serde::Deserialize;
use std::env;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub telegram_bot_token: String,
    pub telegram_chat_id: i64,
    pub encryption_key: String,
    pub max_file_size: usize,
    pub rate_limit_per_minute: u32,
    pub bind_address: String,
    pub allowed_image_types: Vec<String>,
    #[serde(default)]
    pub admin_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let config = Self {
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
                .context("TELEGRAM_BOT_TOKEN environment variable is required")?,
            telegram_chat_id: env::var("TELEGRAM_CHAT_ID")
                .context("TELEGRAM_CHAT_ID environment variable is required")?
                .parse()
                .context("TELEGRAM_CHAT_ID must be a valid integer")?,
            encryption_key: env::var("ENCRYPTION_KEY")
                .context("ENCRYPTION_KEY environment variable is required")?,
            max_file_size: env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| "10485760".to_string()) // 10MB default
                .parse()
                .context("MAX_FILE_SIZE must be a valid integer")?,
            rate_limit_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("RATE_LIMIT_PER_MINUTE must be a valid integer")?,
            bind_address: env::var("BIND_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            allowed_image_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
            ],
            admin_secret: env::var("ADMIN_SECRET").unwrap_or_else(|_| "".to_string()),
        };

        // Validate encryption key length
        let key_bytes = general_purpose::STANDARD.decode(&config.encryption_key)
            .context("ENCRYPTION_KEY must be valid base64")?;
        if key_bytes.len() != 32 {
            return Err(anyhow::anyhow!("ENCRYPTION_KEY must be 32 bytes (256 bits) when decoded"));
        }

        Ok(config)
    }

    pub fn get_encryption_key_bytes(&self) -> Result<[u8; 32]> {
        let key_bytes = general_purpose::STANDARD.decode(&self.encryption_key)
            .context("Failed to decode encryption key")?;
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(key)
    }
}
