//! 双因素认证应用服务

use genies::context::CONTEXT;

use crate::domain::entity::user_2fa_entity::UserTwoFactor;
use crate::domain::repository::user_2fa_repository;
use crate::domain::service::totp_service::TotpService;
use crate::domain::service::second_password_service::SecondPasswordService;
use crate::domain::service::sms_service::SmsService;
use crate::infrastructure::crypto::CryptoUtil;

pub struct TwoFactorAppService;

impl TwoFactorAppService {
    /// 获取用户的 2FA 状态
    pub async fn get_status(user_id: i64) -> Result<Option<UserTwoFactor>, String> {
        let rb = &CONTEXT.rbatis;
        user_2fa_repository::find_by_user_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// 检查 2FA 是否需要（全局启用 && 用户已绑定）
    pub async fn is_2fa_required(user_id: i64) -> Result<bool, String> {
        use crate::application::settings_service::SettingsAppService;

        let two_fa_settings = SettingsAppService::get_2fa_settings().await?;
        if !two_fa_settings.enabled {
            return Ok(false);
        }

        let user_2fa = Self::get_status(user_id).await?;
        Ok(user_2fa.map(|u| u.enabled == 1).unwrap_or(false))
    }

    /// 检查系统是否允许指定 2FA 方式的设置
    async fn check_method_allowed(method: &str) -> Result<(), String> {
        use crate::application::settings_service::SettingsAppService;

        let settings = SettingsAppService::get_2fa_settings().await?;
        if !settings.enabled {
            return Err("系统未启用双因素认证".to_string());
        }
        if !settings.methods.is_empty() && !settings.methods.iter().any(|m| m == method) {
            let label = match method {
                "totp" => "TOTP 验证器",
                "sms" => "短信验证码",
                "second_password" => "二次密码",
                _ => method,
            };
            return Err(format!("系统未启用「{}」方式", label));
        }
        Ok(())
    }

    /// 获取用户可用的 2FA 方式列表
    pub async fn get_available_methods(user_id: i64) -> Result<Vec<String>, String> {
        let user_2fa = Self::get_status(user_id).await?;
        match user_2fa {
            Some(u2f) if u2f.enabled == 1 => Ok(vec![u2f.method]),
            _ => Ok(vec![]),
        }
    }

    /// 发起 TOTP 绑定
    pub async fn setup_totp(user_id: i64, username: &str) -> Result<serde_json::Value, String> {
        Self::check_method_allowed("totp").await?;

        let rb = &CONTEXT.rbatis;

        let (secret, otpauth_url) = TotpService::generate_secret(username, "auth-admin")?;
        let qr_svg = TotpService::generate_qr_svg(&otpauth_url);

        let encryption_key = CryptoUtil::resolve_key(&CONTEXT.config.two_fa_encryption_key);
        let encrypted_secret = CryptoUtil::encrypt(&secret, &encryption_key)?;

        // upsert: 先删后插
        let _ = user_2fa_repository::delete_by_user_id(rb, &user_id).await;

        let two_fa = UserTwoFactor {
            id: None,
            user_id,
            method: "totp".to_string(),
            enabled: 0,
            secret: encrypted_secret,
            phone: String::new(),
            backup_codes: None,
            created_at: None,
            updated_at: None,
        };

        UserTwoFactor::insert(rb, &two_fa)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "secret": secret,
            "otpauth_url": otpauth_url,
            "qr_svg": qr_svg,
        }))
    }

    /// 确认 TOTP 绑定
    pub async fn confirm_totp(user_id: i64, code: &str) -> Result<Vec<String>, String> {
        Self::check_method_allowed("totp").await?;

        let rb = &CONTEXT.rbatis;

        let record = user_2fa_repository::find_by_user_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未找到待确认的 TOTP 绑定，请先发起绑定".to_string())?;

        if record.enabled == 1 {
            return Err("TOTP 已启用".to_string());
        }

        let encryption_key = CryptoUtil::resolve_key(&CONTEXT.config.two_fa_encryption_key);
        let secret = CryptoUtil::decrypt(&record.secret, &encryption_key)?;

        if !TotpService::verify(&secret, code) {
            return Err("TOTP 验证码错误".to_string());
        }

        let backup_codes = TotpService::generate_backup_codes(8);

        let hashed_codes: Vec<String> = backup_codes
            .iter()
            .map(|c| bcrypt::hash(c, bcrypt::DEFAULT_COST).unwrap_or_default())
            .collect();
        let backup_codes_json = serde_json::to_string(&hashed_codes).unwrap_or_default();

        if let Some(id) = record.id {
            user_2fa_repository::update_enabled_and_backup_codes(
                rb, &id, &1i8, &backup_codes_json,
            )
            .await
            .map_err(|e| e.to_string())?;
        }

        Ok(backup_codes)
    }

    /// 关闭 2FA
    pub async fn disable(user_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        user_2fa_repository::delete_by_user_id(rb, &user_id)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// 设置二次密码
    pub async fn setup_second_password(user_id: i64, password: &str) -> Result<(), String> {
        Self::check_method_allowed("second_password").await?;

        let rb = &CONTEXT.rbatis;
        let password_hash = SecondPasswordService::hash_password(password)?;

        let _ = user_2fa_repository::delete_by_user_id(rb, &user_id).await;

        let two_fa = UserTwoFactor {
            id: None,
            user_id,
            method: "second_password".to_string(),
            enabled: 1,
            secret: password_hash,
            phone: String::new(),
            backup_codes: None,
            created_at: None,
            updated_at: None,
        };

        UserTwoFactor::insert(rb, &two_fa)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// 获取备用恢复码
    pub async fn get_backup_codes(user_id: i64) -> Result<Vec<String>, String> {
        let rb = &CONTEXT.rbatis;
        let record = user_2fa_repository::find_by_user_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未启用 2FA".to_string())?;

        let codes_str = record.backup_codes.as_deref().unwrap_or("[]");
        serde_json::from_str::<Vec<String>>(codes_str).map_err(|e| e.to_string())
    }

    /// 重新生成备用恢复码
    pub async fn regenerate_backup_codes(user_id: i64) -> Result<Vec<String>, String> {
        let backup_codes = TotpService::generate_backup_codes(8);

        let hashed_codes: Vec<String> = backup_codes
            .iter()
            .map(|c| bcrypt::hash(c, bcrypt::DEFAULT_COST).unwrap_or_default())
            .collect();
        let backup_codes_json = serde_json::to_string(&hashed_codes).unwrap_or_default();

        let rb = &CONTEXT.rbatis;
        let record = user_2fa_repository::find_by_user_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未启用 2FA".to_string())?;

        if let Some(id) = record.id {
            user_2fa_repository::update_backup_codes(rb, &id, &backup_codes_json)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(backup_codes)
    }

    /// 验证 2FA
    pub async fn verify(user_id: i64, code: &str, method: &str) -> Result<bool, String> {
        let rb = &CONTEXT.rbatis;
        let record = user_2fa_repository::find_by_user_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未绑定 2FA".to_string())?;

        if record.enabled != 1 {
            return Err("2FA 未启用".to_string());
        }

        match method {
            "totp" => {
                let encryption_key = CryptoUtil::resolve_key(&CONTEXT.config.two_fa_encryption_key);
                let secret = CryptoUtil::decrypt(&record.secret, &encryption_key)?;
                if TotpService::verify(&secret, code) {
                    return Ok(true);
                }
                Self::verify_backup_code(&record, code)
            }
            "sms" => {
                SmsService::verify_code(&record.phone, code)
                    .await
                    .map(|_| true)
                    .map_err(|e| e.to_string())
            }
            "second_password" => {
                Ok(SecondPasswordService::verify_password(code, &record.secret))
            }
            _ => Err(format!("不支持的验证方式: {}", method)),
        }
    }

    fn verify_backup_code(record: &UserTwoFactor, code: &str) -> Result<bool, String> {
        let codes_str = record.backup_codes.as_deref().unwrap_or("[]");
        let hashed_codes: Vec<String> =
            serde_json::from_str(codes_str).map_err(|e| e.to_string())?;

        for hashed in &hashed_codes {
            if bcrypt::verify(code, hashed).unwrap_or(false) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 发送短信验证码
    pub async fn send_sms_code(user_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let record = user_2fa_repository::find_by_user_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未绑定短信 2FA".to_string())?;

        if record.phone.is_empty() {
            return Err("未设置手机号码".to_string());
        }

        SmsService::send_code(&record.phone).await?;
        Ok(())
    }

    /// 发起短信 2FA 绑定 — 保存手机号并发送验证码
    pub async fn setup_sms(user_id: i64, phone: &str) -> Result<(), String> {
        Self::check_method_allowed("sms").await?;

        let rb = &CONTEXT.rbatis;

        let phone = phone.trim();
        if phone.is_empty() {
            return Err("手机号码不能为空".to_string());
        }
        if phone.len() < 11 {
            return Err("请输入有效的手机号码".to_string());
        }

        // 先发验证码再保存记录（避免保存后发送失败导致脏记录）
        SmsService::send_code(phone).await?;

        let _ = user_2fa_repository::delete_by_user_id(rb, &user_id).await;

        let two_fa = UserTwoFactor {
            id: None,
            user_id,
            method: "sms".to_string(),
            enabled: 0,
            secret: String::new(),
            phone: phone.to_string(),
            backup_codes: None,
            created_at: None,
            updated_at: None,
        };

        UserTwoFactor::insert(rb, &two_fa)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// 验证短信验证码并启用短信 2FA
    pub async fn verify_sms_setup(user_id: i64, code: &str) -> Result<(), String> {
        Self::check_method_allowed("sms").await?;

        let rb = &CONTEXT.rbatis;
        let record = user_2fa_repository::find_by_user_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未发起短信 2FA 绑定，请先设置手机号".to_string())?;

        if record.enabled == 1 {
            return Err("短信 2FA 已启用".to_string());
        }

        if record.phone.is_empty() {
            return Err("未设置手机号码".to_string());
        }

        SmsService::verify_code(&record.phone, code).await?;

        // 验证通过，启用短信 2FA
        if let Some(id) = record.id {
            user_2fa_repository::update_enabled_and_backup_codes(
                rb, &id, &1i8, "[]",
            )
            .await
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// 管理员强制重置用户 2FA
    pub async fn admin_reset(user_id: i64) -> Result<(), String> {
        Self::disable(user_id).await
    }
}
