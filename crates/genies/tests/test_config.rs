#[cfg(test)]
pub mod config_tests {
    use std::fs::File;
    use std::io::Read;
    use ddd_dapr::config::app_context::ApplicationConfig;
    use ddd_dapr::config_gateway;
    use once_cell::sync::Lazy;

    #[tokio::test]
    async fn config_test() {
        let mut f = File::open("application.yml").unwrap();
        let mut configfile_string = String::new();
        f.read_to_string(&mut configfile_string);

        let yml_data = configfile_string;
        let config = serde_yaml::from_str::<ApplicationConfig>(&yml_data).expect("application.yml read failed!");
        println!("{:?}",config);
        // config_env!(&config,ApplicationConfig);

        // config_env!(ApplicationConfig{debug,server_name});

    }
    #[tokio::test]
    async fn config_file_test() {
        use  ddd_dapr_derive::Config;
        #[derive(Config,Debug,serde::Deserialize)]
        #[config_file("app-test.yml")]
        pub struct TestConfig {
            pub debug: bool,
            pub server_name: String,
            /// 当前服务路由前缀
            pub servlet_path: String,
            ///当前服务地址
            pub server_url: Option<String>,
            /// 用于指定gateway 如果指定了gateway 为合法的 http 协议 所有跨微访问都将通 gateway 进行
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

            pub white_list_api: Vec<String>,

        }
      println!("{:?}",TestConfig::default());
    }


    #[test]
    fn test_config(){
        use std::ops::Deref;
        let a: Lazy<String> = config_gateway!("/sickbed");
        println!("a = {}",a.deref());
    }
}