//! 共享基础设施 — 所有对比测试文件复用
//!
//! 通用测试工具来自 genies_test crate，业务特定配置在此定义。

pub use genies_test::*;

use rbatis::RBatis;
use rbdc_mysql::MysqlDriver;

// ==================== 业务特定配置 ====================

pub fn java_base_url() -> String {
    std::env::var("JAVA_BASE_URL").unwrap_or_else(|_| "http://58.20.184.66:6015/sickbed".to_string())
}

pub fn rust_base_url() -> String {
    std::env::var("RUST_BASE_URL").unwrap_or_else(|_| "http://localhost:8083/sickbed".to_string())
}

/// 测试用科室ID - 可通过环境变量覆盖
pub fn test_dept_id() -> String {
    std::env::var("TEST_DEPT_ID").unwrap_or_else(|_| "7b881e37-1a8e-4446-870e-e3dc1c74c042".to_string())
}

/// 测试用病房ID
pub fn test_ward_id() -> String {
    std::env::var("TEST_WARD_ID").unwrap_or_else(|_| "eaa001bb-2132-429a-a659-1d2bc260d5c6".to_string())
}

/// 测试用床位ID
pub fn test_sickbed_id() -> String {
    std::env::var("TEST_SICKBED_ID").unwrap_or_else(|_| "d497bf2d-fdad-4222-9ce5-b5dbf3165ebe".to_string())
}

/// 测试用扫描号
pub fn test_scan_no() -> String {
    std::env::var("TEST_SCAN_NO").unwrap_or_else(|_| "ward42".to_string())
}

/// 测试用用户ID
pub fn test_user_id() -> String {
    std::env::var("TEST_USER_ID").unwrap_or_else(|_| "8bfc49d5-e077-4908-ac3a-132acac60741".to_string())
}

// ==================== 数据库配置 ====================

/// 数据库连接地址（与 Java/Rust 服务共用同一个数据库）
pub fn database_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://tdnis:Tdcare123for$@58.20.184.66:6012/sickbed_service".to_string())
}

/// 创建一个独立的 RBatis 连接，用于测试脚本直接读写数据库
pub async fn init_test_rbatis() -> RBatis {
    let rb = RBatis::new();
    rb.init(MysqlDriver {}, &database_url()).unwrap();
    rb
}

/// 对 JSON Value 中的数组按指定 key 排序（递归处理嵌套对象中的数组）
pub fn sort_json_arrays(value: &mut serde_json::Value, sort_key: &str) {
    match value {
        serde_json::Value::Array(arr) => {
            arr.sort_by(|a, b| {
                let a_key = a.get(sort_key).and_then(|v| v.as_str()).unwrap_or("");
                let b_key = b.get(sort_key).and_then(|v| v.as_str()).unwrap_or("");
                a_key.cmp(b_key)
            });
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                sort_json_arrays(v, sort_key);
            }
        }
        _ => {}
    }
}

/// 清理测试遗留的脏数据（sickbedNo 以 "CMP-" 开头的记录）
/// 在比对测试前调用，确保 Rust 和 Java 查询结果一致
pub async fn cleanup_test_artifacts(rb: &RBatis) {
    let sql = "DELETE FROM SickbedEntity WHERE sickbedNo LIKE 'CMP-%'";
    match rb.exec(sql, vec![]).await {
        Ok(result) => {
            if result.rows_affected > 0 {
                println!("[cleanup] 已清理 {} 条 CMP 脏数据", result.rows_affected);
            }
        }
        Err(e) => {
            eprintln!("[cleanup] 清理 CMP 脏数据失败: {}", e);
        }
    }
}
