//! 数据库迁移基础设施

use flyway::MigrationRunner;
use flyway_rbatis::RbatisMigrationDriver;
use std::sync::Arc;
use genies::context::CONTEXT;

#[flyway::migrations("migrations")]
pub struct Migrations {}

pub async fn run_migrations() {
    let rbatis = Arc::new(CONTEXT.rbatis.clone());
    let driver = Arc::new(RbatisMigrationDriver::new(rbatis, None));
    let runner = MigrationRunner::new(Migrations {}, driver.clone(), driver.clone(), false);
    match runner.migrate().await {
        Ok(v) => log::info!("[auth-admin] 数据库迁移完成, 版本: {:?}", v),
        Err(e) => log::warn!("[auth-admin] 数据库迁移警告: {}, 继续启动", e),
    }
}
