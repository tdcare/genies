use std::sync::Arc;
use std::time::Duration;
// use async_std::task;
use genies_core::jwt::*;
use serde::{Deserialize, Serialize};
use genies_config::app_config::ApplicationConfig;
use genies_cache::cache_service::CacheService;
use rbatis::RBatis;
// use rbdc::deadpool::managed::{PoolBuilder, Timeouts};
// use rbdc::pool::{ManagerPorxy, Pool, RBDCManager};
use rbdc_mysql::driver::MysqlDriver;
// use deadpool_runtime::Runtime;

// use tracing::{debug, error, info, span, warn, Level};


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RemoteToken {
    pub access_token: String,
}

impl RemoteToken {
   pub fn new() -> Self {
        let config = ApplicationConfig::default();
        Self {
            access_token: futures::executor::block_on(async {
                get_temp_access_token(
                    &config.keycloak_auth_server_url,
                    &config.keycloak_realm,
                    &config.keycloak_resource,
                    &config.keycloak_credentials_secret,
                )
                    .await
            }),
        }
    }
}

pub struct ApplicationContext {
    pub config: ApplicationConfig,
    pub rbatis: RBatis,
    pub cache_service: CacheService,
    pub redis_save_service: CacheService,
    pub keycloak_keys: Keys,
}
impl ApplicationContext {
    /// init database pool
    pub async fn init_pool(&self) {
        //连接数据库
        log::debug!(
            "rbatis pool init ({})...",
            self.config.database_url
        );


        let _ = self.rbatis.get_pool().unwrap().set_max_open_conns(self.config.max_connections as u64);
        let _ = self.rbatis.get_pool().unwrap().set_max_idle_conns(self.config.wait_timeout as u64);
        let _ = self.rbatis.get_pool().unwrap().set_conn_max_lifetime(Some(Duration::from_secs(self.config.max_lifetime as u64)));

        let _ = self.rbatis.get_pool().unwrap().get().await;

        log::info!("rbatis pool init success! pool state = {:?}",
                 self.rbatis.get_pool().expect("pool not init!").state().await
            );

        // let manager = ManagerPorxy::from(
        //     Arc::new(RBDCManager::new(MysqlDriver{}, &self.config.database_url).unwrap()));
        // let inner=Pool::builder(manager.clone())
        //     .runtime(Runtime::Tokio1)
        //     .wait_timeout(Some(Duration::from_secs(self.config.wait_timeout as u64)))
        //     .create_timeout(Some(Duration::from_secs(self.config.create_timeout as u64)))
        //     .recycle_timeout(Some(Duration::from_secs(self.config.max_lifetime as u64)))
        //     .max_size(self.config.max_connections as usize)
        //     .build().unwrap();
        // let pool=Pool{ manager, inner };
        // match  self.rbatis.pool.set(pool){
        //     Ok(_)=>{
        //         log::info!("rbatis pool init success! pool state = {:?}",
        //          self.rbatis.get_pool().expect("pool not init!").state().await
        //     );
        //     },
        //     _=>{log::error!("RBatis 初始化失败 ")}
        // };

    }

    pub fn new() -> Self {
        let config = ApplicationConfig::from_sources("application.yml").unwrap();
        
        ApplicationContext {
            keycloak_keys: futures::executor::block_on(async {
                get_keycloak_keys(&config.keycloak_auth_server_url, &config.keycloak_realm).await
            }),
            rbatis: RBatis::new(),
            cache_service: CacheService::new(&config),
            redis_save_service: CacheService::new_saved(&config),
            config,
        }
    }
}
impl Default for ApplicationContext {
    fn default() -> Self {
        let  config = ApplicationConfig::default();
        ApplicationContext {
            keycloak_keys: futures::executor::block_on(async {
                get_keycloak_keys(&config.keycloak_auth_server_url, &config.keycloak_realm).await
            }),

            rbatis:  RBatis::new(),
            cache_service: CacheService::new(&config),
            redis_save_service: CacheService::new_saved(&config),
            config,
        }
    }
}
//
// //连接数据库
// pub  fn init_rbatis(config: &ApplicationConfig) -> Rbatis {
//     let rbatis = Rbatis::new();
//
//     return rbatis;
// }
