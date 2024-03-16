use  genies_derive::Config;

///服务启动配置
#[derive(Config,Debug,serde::Deserialize)]
pub struct ApplicationConfig {
    pub debug: bool,
    pub server_name:String,
    /// 当前服务路由前缀
    pub servlet_path: String,
    ///当前服务地址
    pub server_url: String,
    /// 用于指定gateway 如果指定了gateway 为合法的 http 协议(以http:// 或 https:// 开头) 所有跨微访问都将通 gateway 进行
    /// 如果gateway 为非法的http 协议 将通Dapr 方式进行访问
    pub gateway: Option<String>,
    ///redis地址
    pub redis_url: String,

    /// 可持久化 redis地址
    pub redis_save_url: String,
    /// 数据库地址
    pub database_url: String,
    /// 数据库连接池参数
    pub max_connections: u32,
    pub min_connections: u32,
    pub wait_timeout: u64,
    pub create_timeout: u64,
    pub max_lifetime: u64,

    // /// 逻辑删除字段
    // pub logic_column: String,
    // pub logic_un_deleted: i64,
    // pub logic_deleted: i64,
    
    
    // ///日志目录 "target/logs/"
    // pub log_dir: String,
    // ///1000
    // pub log_cup: i64,
    // /// "100MB" 日志分割尺寸-单位KB,MB,GB
    // pub log_temp_size: String,
    // /// 日志打包格式可选“”（空-不压缩）“gzip”（gz压缩包）“zip”（zip压缩包）“lz4”（lz4压缩包（非常快））
    // pub log_pack_compress: String,
    // ///日志滚动配置   保留全部:All,按时间保留:KeepTime(Duration),按版本保留:KeepNum(i64)
    // pub log_rolling_type: String,
    ///日志等级
    pub log_level: String,
    
    
    // ///短信缓存队列（mem/redis）
    // pub sms_cache_send_key_prefix: String,
    // ///jwt 秘钥
    // pub jwt_secret: String,
    ///白名单接口
    pub white_list_api: Vec<String>,
    ///权限缓存类型
    pub cache_type: String,
    // ///重试
    // pub login_fail_retry: i64,
    // ///重试等待时间
    // pub login_fail_retry_wait_sec: i64,
    /// keycloak 服务器秘钥
    // pub keycloak_auth_server_certs: String,
    pub keycloak_auth_server_url: String,

    pub keycloak_realm: String,
    pub keycloak_resource: String,
    pub keycloak_credentials_secret: String,

    // /// dapr http 端口
    // pub dapr_http_port: i64,
    // pub dapr_http: String,

    ///Dapr cdc 参数
    pub dapr_pubsub_name: String,
    pub dapr_pub_message_limit: i64,
    pub dapr_cdc_message_period: i64,
    /// 事件消费 参数
    pub processing_expire_seconds: i64,
    pub record_reserve_minutes: i64,
    
    // pub isPollWrite:i64,
    // pub printFlag: Option<i32>,
    // pub test_i64:Option<i64>,
    // pub test_string:Option<String>,
}



