use std::fs::File;
use std::io::prelude::*;
use std::{convert::TryInto, env};
use std::collections::HashMap;

use log::{debug, info};

use crate::config::cdc::CdcConfig;
use std::time::Duration;
use yaml_rust::{Yaml, YamlLoader};

///服务启动配置
pub struct ApplicationConfig {
    pub debug: bool,
    pub server_name:String,
    ///当前服务地址
    pub server_url: String,
    ///redis地址
    pub redis_url: String,

    /// 可持久化 redis地址
    pub redis_save_url: String,
    /// 数据库地址
    pub database_url: String,
    /// 数据库连接池参数
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub max_lifetime: Option<Duration>,
    pub idle_timeout: Option<Duration>,
    pub test_before_acquire: bool,

    /// 逻辑删除字段
    pub logic_column: String,
    pub logic_un_deleted: i64,
    pub logic_deleted: i64,
    ///日志目录 "target/logs/"
    pub log_dir: String,
    ///1000
    pub log_cup: i64,
    /// "100MB" 日志分割尺寸-单位KB,MB,GB
    pub log_temp_size: String,
    /// 日志打包格式可选“”（空-不压缩）“gzip”（gz压缩包）“zip”（zip压缩包）“lz4”（lz4压缩包（非常快））
    pub log_pack_compress: String,
    ///日志滚动配置   保留全部:All,按时间保留:KeepTime(Duration),按版本保留:KeepNum(i64)
    pub log_rolling_type: String,
    ///日志等级
    pub log_level: String,
    ///短信缓存队列（mem/redis）
    pub sms_cache_send_key_prefix: String,
    ///jwt 秘钥
    pub jwt_secret: String,
    ///白名单接口
    pub white_list_api: Vec<String>,
    ///权限缓存类型
    pub cache_type: String,
    ///重试
    pub login_fail_retry: i64,
    ///重试等待时间
    pub login_fail_retry_wait_sec: i64,
    /// keycloak 服务器秘钥
    // pub keycloak_auth_server_certs: String,
    pub keycloak_auth_server_url: String,

    pub keycloak_realm: String,
    pub keycloak_resource: String,
    pub keycloak_credentials_secret: String,

    /// dapr http 端口
    pub dapr_http_port: i64,
    pub dapr_http: String,

    ///Dapr cdc 参数
    pub dapr_pubsub_name: String,
    pub dapr_pub_message_limit: i64,
    pub dapr_cdc_message_period: i64,
    /// 事件消费 参数
    pub processing_expire_seconds: i64,
    pub record_reserve_minutes: i64,

    pub cdc_configs: HashMap<String,CdcConfig>,
}

///默认配置
impl Default for ApplicationConfig {
    fn default() -> Self {
        let mut f = File::open("application.yml").unwrap();
        let mut configfile_string = String::new();
        let _ = f.read_to_string(&mut configfile_string);

        let yml_data = configfile_string;

        let docs = YamlLoader::load_from_str(&yml_data).unwrap();
        //读取配置
        let result = Self {
            debug: get_bool_cfg(&docs, "debug"),
            server_name:get_str_cfg(&docs,"server_name"),
            server_url: get_str_cfg(&docs, "server_url"),
            redis_url: get_str_cfg(&docs, "redis_url"),
            redis_save_url: get_str_cfg(&docs, "redis_save_url"),
            database_url: get_str_cfg(&docs, "database_url"),
            logic_column: get_str_cfg(&docs, "logic_column"),
            logic_un_deleted: get_i64_cfg(&docs, "logic_un_deleted"),
            logic_deleted: get_i64_cfg(&docs, "logic_deleted"),
            log_dir: get_str_cfg(&docs, "log_dir"),
            log_cup: get_i64_cfg(&docs, "log_cup"),
            log_temp_size: get_str_cfg(&docs, "log_temp_size"),
            log_pack_compress: get_str_cfg(&docs, "log_pack_compress"),
            log_rolling_type: get_str_cfg(&docs, "log_rolling_type"),
            log_level: get_str_cfg(&docs, "log_level"),
            sms_cache_send_key_prefix: get_str_cfg(&docs, "sms_cache_send_key_prefix"),
            jwt_secret: get_str_cfg(&docs, "jwt_secret"),
            white_list_api: to_vec_string(
                get_cfg(&docs, "white_list_api").as_vec().unwrap().to_vec(),
            ),
            cache_type: get_str_cfg(&docs, "cache_type"),
            login_fail_retry: get_i64_cfg(&docs, "login_fail_retry"),
            login_fail_retry_wait_sec: get_i64_cfg(&docs, "login_fail_retry_wait_sec"),
            // keycloak_auth_server_certs: get_str_cfg(&docs, "keycloak_auth_server_certs"),
            keycloak_auth_server_url: get_str_cfg(&docs, "keycloak_auth_server_url"),
            keycloak_realm: get_str_cfg(&docs, "keycloak_realm"),
            keycloak_resource: get_str_cfg(&docs, "keycloak_resource"),
            keycloak_credentials_secret: get_str_cfg(&docs, "keycloak_credentials_secret"),

            dapr_http_port: get_i64_cfg(&docs, "dapr_http_port"),
            dapr_http: get_str_cfg(&docs, "dapr_http"),

            dapr_pubsub_name: get_str_cfg(&docs, "dapr_pubsub_name"),
            dapr_pub_message_limit: get_i64_cfg(&docs, "dapr_pub_message_limit"),
            dapr_cdc_message_period: get_i64_cfg(&docs, "dapr_cdc_message_period"),

            processing_expire_seconds: get_i64_cfg(&docs, "processing_expire_seconds"),
            record_reserve_minutes: get_i64_cfg(&docs, "record_reserve_minutes"),

            max_connections: get_u32_cfg(&docs, "max_connections"),
            min_connections: get_u32_cfg(&docs, "min_connections"),
            connect_timeout: Duration::from_secs(get_u32_cfg(&docs, "connect_timeout").into()),
            max_lifetime: Some(Duration::from_secs(
                get_u32_cfg(&docs, "max_lifetime").into(),
            )),
            idle_timeout: Some(Duration::from_secs(
                get_u32_cfg(&docs, "idle_timeout").into(),
            )),
            test_before_acquire: get_bool_cfg(&docs, "test_before_acquire"),

            cdc_configs: to_vec_cdc_config(get_cfg(&docs, "cdc").as_vec().unwrap().to_vec()),
        };

        if result.debug {
            info!("debug_mode is enabled");
        } else {
            info!("release_mode is enabled");
        }

        result
    }
}

/// 获取配置
/// key: 需要获取配置的key
fn get_cfg<'a>(docs: &'a Vec<Yaml>, key: &str) -> &'a Yaml {
    for x in docs {
        match x {
            Yaml::Hash(hash) => {
                let v = hash.get(&Yaml::String(key.to_string()));
                if v.is_some() {
                    return v.unwrap();
                }
            }
            _ => {}
        }
    }
    panic!(" application.yml key: '{}' not exist!", key)
}

fn to_vec_string(arg: Vec<Yaml>) -> Vec<String> {
    let mut arr = vec![];
    for x in arg {
        arr.push(x.as_str().unwrap_or("").to_string());
    }
    return arr;
}
//noinspection ALL
/// 读取CDC 服务 参数
fn to_vec_cdc_config(arg: Vec<Yaml>) -> HashMap<String,CdcConfig> {
    let mut cdc_config_map:HashMap<String,CdcConfig>=HashMap::new();
   // 处理 yaml 文件 中的 参数
    for x in arg {
        debug!("{:?}", x);

       let service_name=  match  x.clone() {
            Yaml::Hash(mut hash) => {
                let mut  iter = hash.entries();
                let  entry = iter.next().unwrap();
                entry.key().as_str().unwrap().to_string()
            },
            _ => {"".to_string()}
        };

        let x1 = vec![x];
        let x2 =get_cfg(&x1, &service_name);
        let x = vec![x2.clone()];

        let  database_url = get_str_cfg(&x, "database_url");
        let  dapr_pubsub_name = get_str_cfg(&x, "dapr_pubsub_name");
        let  dapr_pub_message_limit = get_i64_cfg(&x, "dapr_pub_message_limit");
        let  dapr_cdc_message_period = get_i64_cfg(&x, "dapr_cdc_message_period");
        let clear_message_before_second=get_i64_cfg(&x,"clear_message_before_second");

        let  cdc_config = CdcConfig {
            service_name:Some(service_name.clone()),
            database_url:Some(database_url),
            dapr_pubsub_name:Some(dapr_pubsub_name),
            dapr_pub_message_limit:Some(dapr_pub_message_limit),
            dapr_cdc_message_period:Some(dapr_cdc_message_period),
            clear_message_before_second: Some(clear_message_before_second),
        };
        cdc_config_map.insert(service_name,cdc_config.clone());

    }

  // 处理环境变量中的参数

    // let mut database_url = "".to_string();
    // let mut dapr_pubsub_name = "".to_string();
    // let mut dapr_pub_message_limit = 0;
    // let mut dapr_cdc_message_period = 0;
    let vars=env::vars();
    for (key, value) in vars {
        if key.starts_with("cdc.") || key.starts_with("CDC."){
            let cdc_config:Vec<_>=key.split(".").collect();
            let service_name=cdc_config[1].to_string().to_lowercase();
            let k=cdc_config[2].to_string().to_lowercase();

            if cdc_config_map.get_mut(&service_name).is_none(){
                let  cdc_config = CdcConfig {
                    service_name:Some(service_name.clone()),
                    database_url:None,
                    dapr_pubsub_name:Some("messagebus".to_string()),
                    dapr_pub_message_limit:Some(50),
                    dapr_cdc_message_period:Some(1),
                    clear_message_before_second: Some(172800),
                };
                cdc_config_map.insert(service_name.clone(),cdc_config);
            }

            match  cdc_config_map.get_mut(&service_name) {
                Some(cdc)=>{
                    if k.eq(&"database_url".to_string()){
                        cdc.database_url=Some(value.clone());
                    }
                    if k.eq(&"dapr_pubsub_name".to_string()){
                        cdc.dapr_pubsub_name=Some(value.clone());
                    }
                    if k.eq(&"dapr_pub_message_limit".to_string()){
                        cdc.dapr_pub_message_limit=Some(value.clone().parse::<i64>().unwrap());
                    }
                    if k.eq(&"dapr_cdc_message_period".to_string()){
                        cdc.dapr_cdc_message_period=Some(value.clone().parse::<i64>().unwrap());
                    }
                    if k.eq(&"clear_message_before_second".to_string()){
                        cdc.clear_message_before_second=Some(value.clone().parse::<i64>().unwrap());
                    }
                    },
                // None=>{
                //     let  cdc_config = CdcConfig {
                //         service_name:Some(service_name.clone()),
                //         database_url:None,
                //         dapr_pubsub_name:None,
                //         dapr_pub_message_limit:None,
                //         dapr_cdc_message_period:None,
                //         clear_message_before_second: None,
                //     };
                //     cdc_config_map.insert(service_name,cdc_config.clone());
                // },
               _=>{}
            };
        }


    }
    info!("CDC config loaded for services: {:?}", cdc_config_map.keys().collect::<Vec<_>>());
    return cdc_config_map;
}
/// 根据key 从环境变量中读取 配置参数，如果无环境变量，从yaml 文件中文读取
fn get_str_cfg<'a>(docs: &'a Vec<Yaml>, key: &str) -> String {
    //let str_cfg= env::var(key);
    match env::var(key) {
        Ok(val) => return val,
        Err(_err) => match env::var(key.to_uppercase()) {
            Ok(val) => {
                return val;
            }
            Err(_e) => {
                return get_cfg(docs, key).as_str().unwrap_or("").to_string();
            }
        },
    }
}
fn get_i64_cfg<'a>(docs: &'a Vec<Yaml>, key: &str) -> i64 {
    match env::var(key) {
        Ok(val) => {
            //  println!("{}",key);
            return val.parse::<i64>().unwrap();
        }
        Err(_err) => {
            //   println!("{}",key.to_uppercase());
            match env::var(key.to_uppercase()) {
                Ok(val) => {
                    return val.parse::<i64>().unwrap();
                }
                Err(_e) => {
                    return get_cfg(docs, key).as_i64().unwrap_or(0);
                }
            }
        }
    }
}

fn get_u32_cfg<'a>(docs: &'a Vec<Yaml>, key: &str) -> u32 {
    match env::var(key) {
        Ok(val) => {
            //  println!("{}",key);
            return val.parse::<u32>().unwrap();
        }
        Err(_err) => {
            //   println!("{}",key.to_uppercase());
            match env::var(key.to_uppercase()) {
                Ok(val) => {
                    return val.parse::<u32>().unwrap();
                }
                Err(_e) => {
                    return get_cfg(docs, key).as_i64().unwrap_or(0).try_into().unwrap();
                }
            }
        }
    }
}

fn get_bool_cfg<'a>(docs: &'a Vec<Yaml>, key: &str) -> bool {
    match env::var(key) {
        Ok(val) => {
            return val.parse::<bool>().unwrap();
        }
        Err(_err) => match env::var(key.to_uppercase()) {
            Ok(val) => {
                return val.parse::<bool>().unwrap();
            }
            Err(_e) => {
                return get_cfg(docs, key).as_bool().unwrap_or(true);
            }
        },
    }
}
