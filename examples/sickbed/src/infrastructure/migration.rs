use flyway::MigrationRunner;
use flyway_rbatis::RbatisMigrationDriver;
use genies::context::CONTEXT;
use std::sync::Arc;

#[flyway::migrations("migrations")]
pub struct Migrations {}

pub async fn run_migrations() {
    let rbatis = Arc::new(CONTEXT.rbatis.clone());
    let driver = Arc::new(RbatisMigrationDriver::new(rbatis, None));
    let runner = MigrationRunner::new(
        Migrations {},
        driver.clone(),
        driver.clone(),
        false,
    );
    match runner.migrate().await {
        Ok(v) => log::info!("Sickbed migration completed, latest version: {:?}", v),
        Err(e) => log::warn!("Sickbed migration warning: {}, startup continues", e),
    }
}
