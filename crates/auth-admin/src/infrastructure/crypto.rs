//! AES-256-GCM 加密工具

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use rand::RngCore;

/// 加密工具 — 用于保护 TOTP 密钥等敏感数据
pub struct CryptoUtil;

impl CryptoUtil {
    const NONCE_SIZE: usize = 12; // 96-bit nonce

    /// 加密明文，返回 base64 编码的密文（含 nonce 前缀）
    pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, String> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| format!("AES 初始化失败: {}", e))?;

        let mut nonce_bytes = [0u8; Self::NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("加密失败: {}", e))?;

        // nonce + ciphertext → base64
        let mut combined = Vec::with_capacity(Self::NONCE_SIZE + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(base64::engine::general_purpose::STANDARD.encode(&combined))
    }

    /// 解密 base64 编码的密文
    pub fn decrypt(ciphertext_b64: &str, key: &[u8; 32]) -> Result<String, String> {
        let combined = base64::engine::general_purpose::STANDARD
            .decode(ciphertext_b64)
            .map_err(|e| format!("Base64 解码失败: {}", e))?;

        if combined.len() < Self::NONCE_SIZE + 16 {
            return Err("密文数据不完整".to_string());
        }

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| format!("AES 初始化失败: {}", e))?;

        let nonce = Nonce::from_slice(&combined[..Self::NONCE_SIZE]);
        let ciphertext = &combined[Self::NONCE_SIZE..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("解密失败: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| format!("UTF-8 解码失败: {}", e))
    }

    /// 从 hex 字符串生成 32 字节密钥，或自动生成随机密钥
    pub fn resolve_key(config_key: &str) -> [u8; 32] {
        let trimmed = config_key.trim();
        if trimmed.len() >= 64 {
            // 尝试从 hex 解码
            let bytes: Vec<u8> = (0..trimmed.len())
                .step_by(2)
                .filter_map(|i| u8::from_str_radix(&trimmed[i..i + 2], 16).ok())
                .collect();
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return key;
            }
        }

        // 自动生成随机密钥（服务重启后无法解密旧的 TOTP 密钥，但可接受）
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        log::warn!("[Crypto] 2FA 加密密钥未配置或格式无效，已自动生成随机密钥（服务重启后旧密钥失效）");
        key
    }
}
