

use crate::config::app_config::*;

pub fn init_log(config: &ApplicationConfig) {


    let env_filter_config=&config.log_level;

    tracing_subscriber::fmt()
        // .with_max_level(LevelFilter::DEBUG)
        // .with_env_filter(EnvFilter::from_default_env())
        .with_env_filter( env_filter_config)
        .init();

   
}





