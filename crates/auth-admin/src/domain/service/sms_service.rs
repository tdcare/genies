//! 短信验证码服务 — SMS 网关 trait + 开发用日志实现

use std::time::Duration;
use rand::Rng;
use genies::context::CONTEXT;

const SMS_CACHE_PREFIX: &str = "sms:2fa:";
const SMS_TTL: Duration = Duration::from_secs(300); // 5 分钟

/// SMS 网关 trait（生产环境可替换为阿里云/腾讯云等实现）
#[async_trait::async_trait]
pub trait SmsGateway: Send + Sync {
    async fn send(&self, phone: &str, code: &str) -> Result<(), String>;
}

/// 开发用 SMS 网关 — 仅打印日志
pub struct LogOnlySmsGateway;

#[async_trait::async_trait]
impl SmsGateway for LogOnlySmsGateway {
    async fn send(&self, phone: &str, code: &str) -> Result<(), String> {
        log::info!("[SMS-Dev] 向 {} 发送验证码: {}", phone, code);
        Ok(())
    }
}

pub struct SmsService;

impl SmsService {
    /// 生成 6 位验证码并"发送"（开发模式仅打印日志）
    pub async fn send_code(phone: &str) -> Result<String, String> {
        let code: String = (0..6)
            .map(|_| rand::thread_rng().gen_range('0'..='9'))
            .collect();

        // 缓存验证码
        let cache_key = format!("{}{}", SMS_CACHE_PREFIX, phone);
        CONTEXT.cache_service
            .set_string_ex(&cache_key, &code, Some(SMS_TTL))
            .await
            .map_err(|e| format!("缓存失败: {}", e))?;

        // 发送（开发环境仅打印日志）
        let gateway = LogOnlySmsGateway;
        gateway.send(phone, &code).await?;

        Ok(code)
    }

    /// 验证短信验证码（一次一用）
    pub async fn verify_code(phone: &str, code: &str) -> Result<(), String> {
        let cache_key = format!("{}{}", SMS_CACHE_PREFIX, phone);
        let stored = CONTEXT.cache_service
            .get_string(&cache_key)
            .await
            .unwrap_or_default();

        let _ = CONTEXT.cache_service.del_string(&cache_key).await;

        if stored.is_empty() {
            return Err("验证码已过期".to_string());
        }

        if stored == code.trim() {
            Ok(())
        } else {
            Err("验证码错误".to_string())
        }
    }
}
