//! 数据库迁移模块
//!
//! 使用 flyway-rs 框架管理数据库 schema 迁移
//! 迁移文件位于 `crates/auth/migrations/` 目录

use std::sync::Arc;
use flyway::MigrationRunner;
use flyway_rbatis::RbatisMigrationDriver;
use genies::context::CONTEXT;

/// 迁移文件集合
///
/// 使用 `#[flyway::migrations]` 宏自动加载指定目录下的 SQL 迁移文件
/// 路径是相对于 crate root 的
#[flyway::migrations("migrations")]
pub struct Migrations {}

/// 执行数据库迁移
///
/// 在应用启动时调用此函数，自动执行所有未运行的迁移脚本
/// 迁移执行顺序由文件名前缀（如 V1__, V2__）决定
///
/// # Panics
///
/// 如果迁移执行失败，将 panic 并打印错误信息
pub async fn run_migrations() {
    // 从全局上下文获取 RBatis 实例并包装为 Arc
    let rbatis = Arc::new(CONTEXT.rbatis.clone());
    
    // 创建 RBatis 迁移驱动
    // 第二个参数指定自定义的 schema history 表名，避免与业务模块的迁移表冲突
    let driver = Arc::new(RbatisMigrationDriver::new(rbatis, Some("auth_flyway_migrations")));
    
    // 创建迁移运行器
    // driver 同时作为执行器和历史记录器
    // 第四个参数 fail_continue: false，生产环境保持严格模式
    // V5 的幂等性通过语句级 --! may_fail: true 注解处理
    let runner = MigrationRunner::new(
        Migrations {},
        driver.clone(),
        driver.clone(),
        false,
    );
    
    // 执行迁移，失败时仅输出警告日志，不 panic
    match runner.migrate().await {
        Ok(v) => log::info!("Auth migration completed, latest version: {:?}", v),
        Err(e) => log::warn!("Auth migration warning: {}, startup continues", e),
    }
}
