//! 二次密码服务

pub struct SecondPasswordService;

impl SecondPasswordService {
    /// 设置二次密码（返回 bcrypt 哈希）
    pub fn hash_password(password: &str) -> Result<String, String> {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("密码加密失败: {}", e))
    }

    /// 验证二次密码
    pub fn verify_password(password: &str, hash: &str) -> bool {
        bcrypt::verify(password, hash).unwrap_or(false)
    }
}
