/*
 * @Author: tzw
 * @Date: 2021-10-17 21:43:46
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-02 10:49:01
 */
#![warn(non_snake_case)]

use self::app_context::*;
pub mod app_config;
pub mod app_context;
pub mod log_config;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    //服务运行环境，数据库连接、缓存连接、配置参数等
    pub  static ref  CONTEXT: ApplicationContext = ApplicationContext::default();
    //存放跨微服务访问 token
    pub  static ref  REMOTE_TOKEN: Mutex<RemoteToken> = Mutex::new(RemoteToken::new());

    //服务内部状态信息，如k8s部署 所需要的就绪、存活等状态信息
    pub static ref SERVICE_STATUS:Mutex<HashMap<String,bool>>=Mutex::new(
        {
            let mut map = HashMap::new();
            map.insert("readinessProbe".to_string(), true);
            map.insert("livenessProbe".to_string(),true);
            map
            }
    );
}

