use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use crate::{error::{AppError, Result}, models::FileReference};

pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        Self { cipher }
    }

    /// Encrypt image data
    pub fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut ciphertext = self
            .cipher
            .encrypt(nonce, data)
            .map_err(|e| AppError::EncryptionError(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.append(&mut ciphertext);
        
        Ok(result)
    }

    /// Decrypt image data
    pub fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            return Err(AppError::EncryptionError("Invalid encrypted data".to_string()));
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::EncryptionError(e.to_string()))?;

        Ok(plaintext)
    }

    /// Encrypt file reference for URL-safe ID
    pub fn encrypt_file_reference(&self, file_ref: &FileReference) -> Result<String> {
        let json_data = serde_json::to_vec(file_ref)
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let nonce = Nonce::from_slice(&file_ref.nonce);
        let ciphertext = self
            .cipher
            .encrypt(nonce, json_data.as_slice())
            .map_err(|e| AppError::EncryptionError(e.to_string()))?;

        // Combine nonce and ciphertext
        let mut combined = file_ref.nonce.to_vec();
        combined.extend_from_slice(&ciphertext);

        // Base64 URL-safe encoding
        Ok(general_purpose::URL_SAFE_NO_PAD.encode(&combined))
    }

    /// Decrypt file reference from URL-safe ID
    pub fn decrypt_file_reference(&self, encrypted_id: &str) -> Result<FileReference> {
        let combined = general_purpose::URL_SAFE_NO_PAD.decode(encrypted_id)
            .map_err(|_| AppError::InvalidImageId)?;

        if combined.len() < 12 {
            return Err(AppError::InvalidImageId);
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| AppError::InvalidImageId)?;

        let file_ref: FileReference = serde_json::from_slice(&plaintext)
            .map_err(|_| AppError::InvalidImageId)?;

        Ok(file_ref)
    }

    /// Generate a secure random key
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    /// Hash data using SHA-256
    pub fn hash_data(data: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt_data() {
        let key = CryptoService::generate_key();
        let crypto = CryptoService::new(&key);
        let data = b"Hello, World!";
        
        let encrypted = crypto.encrypt_data(data).unwrap();
        let decrypted = crypto.decrypt_data(&encrypted).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }
    
    #[test]
    fn test_encrypt_decrypt_file_reference() {
        let key = CryptoService::generate_key();
        let crypto = CryptoService::new(&key);
        let file_ref = FileReference::new(
            "test_file_id".to_string(),
            12345,
            1024,
            "image/jpeg".to_string(),
        );
        
        let encrypted_id = crypto.encrypt_file_reference(&file_ref).unwrap();
        let decrypted_ref = crypto.decrypt_file_reference(&encrypted_id).unwrap();
        
        assert_eq!(file_ref.file_id, decrypted_ref.file_id);
        assert_eq!(file_ref.message_id, decrypted_ref.message_id);
        assert_eq!(file_ref.file_size, decrypted_ref.file_size);
        assert_eq!(file_ref.mime_type, decrypted_ref.mime_type);
    }
}
