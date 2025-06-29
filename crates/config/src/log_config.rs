
use  genies_derive::ConfigCore;


/// 日志级别
#[derive(ConfigCore,Debug,serde::Deserialize)]
pub struct LogConfig {
    pub log_level:String,
}

pub fn init_log() {

    let env_filter_config=LogConfig::from_sources("./application.yml").unwrap().log_level;

    tracing_subscriber::fmt()
        // .with_max_level(LevelFilter::DEBUG)
        // .with_env_filter(EnvFilter::from_default_env())
        .with_env_filter( env_filter_config)
        .init();


}

