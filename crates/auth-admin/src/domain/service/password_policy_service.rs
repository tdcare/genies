//! 密码强度策略验证服务

use crate::application::settings_service::{PasswordPolicySettings, SettingsAppService};

pub struct PasswordPolicyService;

impl PasswordPolicyService {
    /// 根据密码策略校验密码强度
    /// 返回失败时包含所有不满足规则的描述列表
    pub fn validate(password: &str, policy: &PasswordPolicySettings) -> Result<(), Vec<String>> {
        let mut errors: Vec<String> = Vec::new();

        // 1. 最小长度
        let len = password.chars().count();
        if len < policy.min_length as usize {
            errors.push(format!("密码长度不能少于{}位", policy.min_length));
        }

        // 2. 大写字母
        if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            errors.push("密码需包含至少一个大写字母".to_string());
        }

        // 3. 小写字母
        if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            errors.push("密码需包含至少一个小写字母".to_string());
        }

        // 4. 数字
        if policy.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("密码需包含至少一个数字".to_string());
        }

        // 5. 特殊字符
        if policy.require_special {
            let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;':\",./<>?`~".contains(c));
            if !has_special {
                errors.push("密码需包含至少一个特殊字符(!@#$%^&*等)".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 便捷方法：自动获取配置并校验
    pub async fn validate_with_policy(password: &str) -> Result<(), String> {
        let policy = SettingsAppService::get_password_policy().await?;

        // 如果最小长度为0，表示不启用密码策略
        if policy.min_length == 0 {
            return Ok(());
        }

        match Self::validate(password, &policy) {
            Ok(()) => Ok(()),
            Err(errors) => Err(errors.join("；")),
        }
    }
}
