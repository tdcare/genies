use std::sync::Once;
// use async_std::task;
use genies_core::jwt::*;
use serde::{Deserialize, Serialize};
use genies_config::app_config::ApplicationConfig;
use genies_cache::cache_service::CacheService;

use rbatis::RBatis;
// use rbdc::deadpool::managed::{PoolBuilder, Timeouts};
// use rbdc::pool::{ManagerPorxy, Pool, RBDCManager};
// use rbdc_mysql::driver::MysqlDriver;
// use deadpool_runtime::Runtime;

// use tracing::{debug, error, info, span, warn, Level};


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RemoteToken {
    pub access_token: String,
}

impl RemoteToken {
   pub fn new() -> Self {
        let config = ApplicationConfig::from_sources("./application.yml").unwrap();
        let url = config.keycloak_auth_server_url.clone();
        let realm = config.keycloak_realm.clone();
        let resource = config.keycloak_resource.clone();
        let secret = config.keycloak_credentials_secret.clone();
        Self {
            access_token: std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    get_temp_access_token(&url, &realm, &resource, &secret)
                        .await
                        .unwrap_or_else(|e| {
                            log::error!("Failed to get temp access token: {}", e);
                            String::new()
                        })
                })
            }).join().unwrap(),
        }
    }
}

pub struct ApplicationContext {
    pub config: ApplicationConfig,
    pub rbatis: RBatis,
    pub cache_service: CacheService,
    pub redis_save_service: CacheService,
    pub keycloak_keys: Keys,
    db_init_once: Once,  // 线程安全的初始化标志
}

/// 尝试在 Redis 中注册一个雪花算法槽位
async fn try_register_slot(cache: &CacheService, server_name: &str) -> Option<(i32, String)> {
    for i in 0..1024i32 {
        let key = format!("snowflake:slot:{}:{}", server_name, i);
        match cache.set_string_ex_nx(&key, "1", Some(std::time::Duration::from_secs(3600))).await {
            Ok(true) => return Some((i, key)),
            _ => continue,
        }
    }
    None
}

/// 按优先级解析 worker_id：Redis 槽位 → K8s HOSTNAME → 配置 → 兜底
fn resolve_worker_id(config: &ApplicationConfig, cache: &CacheService) -> i32 {
    // 1. Redis 槽位注册（直接复用传入的 cache）
    if config.cache_type == "redis" {
        // 安全地执行 async 代码，兼容 "已在 tokio runtime 中" 和 "未在 runtime 中" 两种场景
        let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // 已在 tokio runtime 中（如 #[tokio::main]），使用 block_in_place 避免嵌套 panic
            tokio::task::block_in_place(|| {
                handle.block_on(try_register_slot(cache, &config.server_name))
            })
        } else {
            // 不在 tokio runtime 中，创建临时 runtime
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(try_register_slot(cache, &config.server_name))
        };

        if let Some((id, key)) = result {
            log::info!("Registered snowflake worker_id via Redis slot: {}", id);
            // 启动后台续约任务
            // 续约 30 分钟后才首次执行，此时全局 CONTEXT 已初始化完成，直接复用其 cache_service
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(30 * 60)).await;
                        match crate::CONTEXT.cache_service.set_string_ex(&key, "1", Some(std::time::Duration::from_secs(3600))).await {
                            Ok(_) => log::debug!("Renewed snowflake slot TTL: {}", key),
                            Err(e) => log::warn!("Failed to renew snowflake slot: {}", e),
                        }
                    }
                });
            });
            return id;
        }
    }
    // 2. K8s HOSTNAME
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        if let Some(id) = hostname.rsplit('-').next().and_then(|s| s.parse::<i32>().ok()) {
            let worker_id = id % 1024;
            log::info!("Using K8s pod ordinal as machine_id: {}", worker_id);
            return worker_id;
        }
    }
    // 3. 配置文件
    if let Some(id) = config.machine_id {
        log::info!("Using configured machine_id: {}", id);
        return id as i32;
    }
    // 4. 兜底
    log::warn!("Using fallback machine_id: 1");
    1
}

/// 根据数据库 URL scheme 创建对应的驱动实例
fn create_db_driver(url: &str) -> Box<dyn rbdc::db::Driver> {
    let scheme = url.split("://").next().unwrap_or("");
    match scheme {
        #[cfg(feature = "mysql")]
        "mysql" => Box::new(rbdc_mysql::driver::MysqlDriver {}),
        
        #[cfg(feature = "postgres")]
        "postgres" | "postgresql" => Box::new(rbdc_pg::driver::PgDriver {}),
        
        #[cfg(feature = "sqlite")]
        "sqlite" => Box::new(rbdc_sqlite::driver::SqliteDriver {}),
        
        #[cfg(feature = "mssql")]
        "mssql" | "sqlserver" => Box::new(rbdc_mssql::driver::MssqlDriver {}),
        
        #[cfg(feature = "oracle")]
        "oracle" => Box::new(rbdc_oracle::driver::OracleDriver {}),
        
        #[cfg(feature = "tdengine")]
        "taos" | "taos+ws" => Box::new(rbdc_tdengine::driver::TaosDriver {}),
        
        _ => panic!("Unsupported database scheme '{}'. Check database_url or enable the corresponding feature flag.", scheme),
    }
}
impl ApplicationContext {
    /// 初始化数据库连接池，根据 database_url 自动选择驱动
    pub async fn init_database(&self) {
        self.db_init_once.call_once(|| {
            let driver = create_db_driver(&self.config.database_url);
            log::info!("rbatis database init ({})...", self.config.database_url);
            let _ = self.rbatis.init(driver, &self.config.database_url).unwrap();
            
            let _ = self.rbatis.get_pool().unwrap().set_max_open_conns(self.config.max_connections as u64);
            let _ = self.rbatis.get_pool().unwrap().set_max_idle_conns(self.config.wait_timeout as u64);
            let _ = self.rbatis.get_pool().unwrap().set_conn_max_lifetime(Some(std::time::Duration::from_secs(self.config.max_lifetime)));
        });
        
        // 异步获取连接验证放在 call_once 外面（每次都可以验证）
        let _ = self.rbatis.get_pool().unwrap().get().await;
        
        log::info!("rbatis database init success! pool state = {:?}", self.rbatis.get_pool().unwrap().state().await);
    }

    /// 初始化数据库连接池（已废弃，请使用 init_database）
    #[deprecated(note = "Use init_database() instead")]
    pub async fn init_mysql(&self) {
        self.init_database().await;
    }

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

    pub fn new() -> Self {
        let config = ApplicationConfig::from_sources("./application.yml").unwrap();
        log::debug!("config = {:?}", config);
       
        let auth_url = config.keycloak_auth_server_url.clone();
        let auth_realm = config.keycloak_realm.clone();
        
        let cache_service = CacheService::new(&config);
        let redis_save_service = CacheService::new_saved(&config);
        
        // 初始化雪花 ID 生成器
        let machine_id = resolve_worker_id(&config, &cache_service);
        genies_core::id_gen::init(machine_id, 1);
        
        ApplicationContext {
            keycloak_keys: std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    get_keycloak_keys(&auth_url, &auth_realm).await
                })
            }).join().unwrap()
                .expect("Failed to get keycloak keys"),
            rbatis: RBatis::new(),
            cache_service,
            redis_save_service,
            config,
            db_init_once: Once::new(),
        }
    }
}
impl Default for ApplicationContext {
    fn default() -> Self {
           Self::new()             
    }
}
//
// //连接数据库
// pub  fn init_rbatis(config: &ApplicationConfig) -> Rbatis {
//     let rbatis = Rbatis::new();
//
//     return rbatis;
// }
