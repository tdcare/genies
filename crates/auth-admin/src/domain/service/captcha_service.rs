//! 验证码服务 — 生成扭曲字符图片并验证

use std::time::Duration;

use captcha::filters::{Dots, Noise, Wave};
use captcha::Captcha;
use genies::context::CONTEXT;

const CAPTCHA_TTL: Duration = Duration::from_secs(300); // 5分钟有效期
const CAPTCHA_CACHE_PREFIX: &str = "captcha:";

pub struct CaptchaService;

impl CaptchaService {
    /// 生成验证码图片
    fn generate() -> (String, String, String) {
        let mut c = Captcha::new();
        c.add_chars(4)
            .apply_filter(Noise::new(0.1))
            .apply_filter(Wave::new(1.0, 10.0).horizontal())
            .apply_filter(Wave::new(1.0, 10.0).vertical())
            .apply_filter(Dots::new(5))
            .view(160, 60);

        let text = c.chars_as_string().to_lowercase();
        let png_data = c.as_png().expect("captcha PNG generation should not fail");
        let image_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &png_data,
        );

        let captcha_id = uuid::Uuid::new_v4().to_string();

        (captcha_id, text, image_base64)
    }

    /// 生成并缓存验证码
    pub async fn generate_cached() -> (String, String) {
        let (captcha_id, text, image_base64) = Self::generate();

        let cache_key = format!("{}{}", CAPTCHA_CACHE_PREFIX, captcha_id);
        match CONTEXT.cache_service
            .set_string_ex(&cache_key, &text, Some(CAPTCHA_TTL))
            .await
        {
            Ok(v) => tracing::debug!("[captcha] set captcha key={} val={} result={}", cache_key, text, v),
            Err(e) => tracing::error!("[captcha] set captcha FAILED key={} err={}", cache_key, e),
        }

        (captcha_id, image_base64)
    }

    /// 验证验证码（一次一用，验证后无论成功与否都删除）
    pub async fn verify(captcha_id: &str, user_text: &str) -> Result<(), &'static str> {
        let cache_key = format!("{}{}", CAPTCHA_CACHE_PREFIX, captcha_id);

        // 从缓存获取
        let stored_result = CONTEXT.cache_service
            .get_string(&cache_key)
            .await;
        
        let stored = match &stored_result {
            Ok(s) => {
                tracing::debug!("[captcha] get captcha key={} val={}", cache_key, s);
                s.clone()
            }
            Err(e) => {
                tracing::error!("[captcha] get captcha FAILED key={} err={}", cache_key, e);
                String::new()
            }
        };

        // 删除缓存（一次一用）
        let _ = CONTEXT.cache_service.del_string(&cache_key).await;

        if stored.is_empty() {
            return Err("验证码已过期，请刷新重试");
        }

        // 大小写不敏感比较
        if stored.to_lowercase() == user_text.to_lowercase().trim() {
            Ok(())
        } else {
            Err("验证码错误")
        }
    }
}
