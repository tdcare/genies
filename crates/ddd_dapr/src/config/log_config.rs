// use crate::config::CONTEXT;
// use fast_log::consts::LogSize;
// use fast_log::Config;
// use fast_log::FastLogFormat;
// use fast_log::plugin::file_split::{FileSplitAppender, Packer, RollingType};
// use fast_log::plugin::packer::{GZipPacker, LZ4Packer, LogPacker};
// use std::time::Duration;
// use log::LevelFilter;

use  ddd_dapr_derive::Config;

// use tracing::{info,Level};
// use tracing_subscriber::FmtSubscriber;

/// 日志级别
#[derive(Config,Debug,serde::Deserialize)]
pub struct LogConfig {
    pub log_level:String,
}

pub fn init_log() {
    //create log dir
    // std::fs::create_dir_all(&config.log_dir);

    //init fast log
    // let mut cfg = Config::new()
    //     .format(FastLogFormat::new().set_display_line_level(LevelFilter::Error))
    //     .level(str_to_log_level( &config.log_level))
    //     .custom(FileSplitAppender::new(
    //          &config.log_dir,
    //         str_to_temp_size( &config.log_temp_size),
    //         str_to_rolling( &config.log_rolling_type),
    //         choose_packer( &config.log_pack_compress),
    //     ));
    // k8s 环境下，日志统一输出到标准输出，然后再由统一的日志收集器处理
    // if config.debug {
    //    let cfg = Config::new().chan_len(None).level(str_to_log_level( &config.log_level))
    //              .console();
    // // // }
    // match  fast_log::init(cfg){
    //     Ok(_)=>{},
    //     _=>{
    //         log::info!("日志组件初始化出错")
    //     }
    // };

    let env_filter_config=LogConfig::default().log_level;

    tracing_subscriber::fmt()
        // .with_max_level(LevelFilter::DEBUG)
        // .with_env_filter(EnvFilter::from_default_env())
        .with_env_filter( env_filter_config)
        .init();

    // if config.debug == false {
    //     println!(" release_mode is up! [file_log] open,[console_log] disabled!");
    // }
    // let subscriber = tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::ERROR)
    //     .compact()
    //     .finish();
    //
    // // tracing::subscriber::with_default(subscriber, || {
    // //     info!("This will be logged to stdout");
    // // });
    // tracing::subscriber::set_global_default(subscriber)
    //     .expect("setting default subscriber failed");
    // tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

}

// fn choose_packer(packer: &str) -> Box<dyn Packer> {
//     match packer {
//         "lz4" => Box::new(LZ4Packer {}),
//         "zip" => Box::new(LZ4Packer {}),
//         "gzip" => Box::new(GZipPacker {}),
//         _ => Box::new(LogPacker {}),
//     }
// }
//
// fn str_to_temp_size(arg: &str) -> LogSize {
//     match arg {
//         arg if arg.ends_with("MB") => {
//             let end = arg.find("MB").unwrap();
//             let num = arg[0..end].to_string();
//             LogSize::MB(num.parse::<usize>().unwrap())
//         }
//         arg if arg.ends_with("KB") => {
//             let end = arg.find("KB").unwrap();
//             let num = arg[0..end].to_string();
//             LogSize::KB(num.parse::<usize>().unwrap())
//         }
//         arg if arg.ends_with("GB") => {
//             let end = arg.find("GB").unwrap();
//             let num = arg[0..end].to_string();
//             LogSize::GB(num.parse::<usize>().unwrap())
//         }
//         _ => LogSize::MB(100),
//     }
// }
//
// fn str_to_rolling(arg: &str) -> RollingType {
//     match arg {
//         arg if arg.starts_with("KeepNum(") => {
//             let end = arg.find(")").unwrap();
//             let num = arg["KeepNum(".len()..end].to_string();
//             RollingType::KeepNum(num.parse::<i64>().unwrap())
//         }
//         arg if arg.starts_with("KeepTime(") => {
//             let end = arg.find(")").unwrap();
//             let num = arg["KeepTime(".len()..end].to_string();
//             RollingType::KeepTime(Duration::from_secs(num.parse::<u64>().unwrap()))
//         }
//         _ => RollingType::All,
//     }
// }
//
// fn str_to_log_level(arg: &str) -> log::LevelFilter {
//     return match arg {
//         "warn" => log::LevelFilter::Warn,
//         "error" => log::LevelFilter::Error,
//         "trace" => log::LevelFilter::Trace,
//         "info" => log::LevelFilter::Info,
//         "debug" => log::LevelFilter::Debug,
//         _ => log::LevelFilter::Info,
//     };
// }
// pub fn str_to_tracing_log_level(arg: &str) -> tracing::Level {
//     return match arg {
//         "warn" => tracing::Level::WARN,
//         "error" => tracing::Level::ERROR,
//         "trace" => tracing::Level::TRACE,
//         "info" => tracing::Level::INFO,
//         "debug" => tracing::Level::DEBUG,
//         _ => tracing::Level::INFO,
//     };
//
// }
