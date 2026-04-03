use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use rbatis::RBatis;
use rbdc_mysql::driver::MysqlDriver;

use crate::config::cdc::CdcConfig;
use crate::error::*;

use self::app_context::ServiceContext;

pub mod app_config;
pub mod app_context;
pub mod cdc;
pub mod log;

pub static CONTEXT: LazyLock<ServiceContext> = LazyLock::new(ServiceContext::default);

pub static POOLS: LazyLock<HashMap<String, RBatis>> = LazyLock::new(|| {
    make_pools(&CONTEXT.config.cdc_configs)
        .expect("Failed to initialize database pools for CDC")
});

pub static SERVICE_STATUS: LazyLock<Mutex<HashMap<String, bool>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert("readinessProbe".to_string(), true);
    map.insert("livenessProbe".to_string(), true);
    Mutex::new(map)
});

fn make_pools(cdc_configs: &HashMap<String, CdcConfig>) -> Result<HashMap<String, RBatis>> {
    let mut pools: HashMap<String, RBatis> = HashMap::new();
    for (service_name, cdc_config) in cdc_configs {
        let rb: RBatis = RBatis::new();
        let pool = rb.init(MysqlDriver {}, &*cdc_config.database_url.as_ref().unwrap());
        if pool.is_ok() {
            pools.insert(service_name.clone(), rb);
        } else {
            panic!(
                "Database connection failed: {:?}\nCheck configuration: {:?}",
                &pool, &cdc_config
            );
        }
    }
    Ok(pools)
}
