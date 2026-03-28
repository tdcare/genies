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
    // 第二个参数 None 表示使用默认的 schema history 表名
    let driver = Arc::new(RbatisMigrationDriver::new(rbatis, None));
    
    // 创建迁移运行器
    // driver 同时作为执行器和历史记录器
    // 第四个参数 fail_continue: false 表示遇到失败时停止
    let runner = MigrationRunner::new(
        Migrations {},
        driver.clone(),
        driver.clone(),
        false,
    );
    
    // 执行迁移
    runner.migrate().await.expect("数据库迁移失败");
}
