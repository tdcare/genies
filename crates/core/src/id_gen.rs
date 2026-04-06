//! 统一雪花 ID 生成器
//!
//! 启动时由 `ApplicationContext` 自动初始化 worker_id，
//! 业务代码通过 `next_id()` 获取全局唯一 ID。

use std::sync::{Mutex, OnceLock};
use snowflake::SnowflakeIdBucket;

static ID_GEN: OnceLock<Mutex<SnowflakeIdBucket>> = OnceLock::new();

/// 初始化雪花 ID 生成器（启动时由 ApplicationContext 调用一次）
///
/// - `machine_id`: 机器ID（0..1023）
/// - `datacenter_id`: 数据中心ID（默认 1）
pub fn init(machine_id: i32, datacenter_id: i32) {
    ID_GEN.get_or_init(|| {
        log::info!("Snowflake ID generator initialized: machine_id={}, datacenter_id={}", machine_id, datacenter_id);
        Mutex::new(SnowflakeIdBucket::new(machine_id, datacenter_id))
    });
}

/// 生成全局唯一 ID（String 类型，适合直接存入数据库 VARCHAR 字段）
pub fn next_id() -> String {
    ID_GEN.get()
        .expect("ID generator not initialized, call genies_core::id_gen::init() first")
        .lock()
        .unwrap()
        .get_id()
        .to_string()
}
