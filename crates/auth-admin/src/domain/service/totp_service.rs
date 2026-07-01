//! TOTP 双因素认证服务

use totp_rs::{Algorithm, TOTP, Secret};
use qrcode::QrCode;
use qrcode::render::svg;

pub struct TotpService;

impl TotpService {
    /// 生成 TOTP 密钥和 otpauth URL（供生成 QR 码）
    pub fn generate_secret(username: &str, issuer: &str) -> Result<(String, String), String> {
        let secret_obj = Secret::generate_secret();
        // 在消费之前先获取 base32 编码
        let secret_base32 = secret_obj.to_encoded().to_string();

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_obj.to_bytes().map_err(|e| e.to_string())?,
            Some(issuer.to_string()),
            username.to_string(),
        ).map_err(|e| e.to_string())?;

        let otpauth_url = totp.get_url();

        Ok((secret_base32, otpauth_url))
    }

    /// 生成 QR 码 SVG 字符串
    pub fn generate_qr_svg(data: &str) -> String {
        let code = QrCode::new(data).unwrap();
        code.render::<svg::Color>()
            .min_dimensions(200, 200)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build()
    }

    /// 从存储的密钥验证 TOTP 码
    pub fn verify(secret_base32: &str, code: &str) -> bool {
        match TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            Secret::Encoded(secret_base32.to_string()).to_bytes().unwrap_or_default(),
            None,
            "".to_string(),
        ) {
            Ok(totp) => totp.check_current(code).unwrap_or(false),
            Err(_) => false,
        }
    }

    /// 生成备用恢复码（8 位十六进制）
    pub fn generate_backup_codes(count: usize) -> Vec<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..count)
            .map(|_| format!("{:08x}", rng.gen::<u32>()))
            .collect()
    }
}
