#[cfg(test)]
pub mod jwt_tests {
    use ddd_dapr::auth::jwt::{get_keycloak_keys, get_temp_access_token, Keys};
    use ddd_dapr::config::app_config::ApplicationConfig;
    use ddd_dapr::config::CONTEXT;
    use ddd_dapr::config::log_config::init_log;

    #[tokio::test]
    async fn get_keycloak_keys_test() {
        init_log();
        let  config = ApplicationConfig::default();

        // reqwest_test().await;
        let url=config.keycloak_auth_server_url.clone();
        let url=url.as_str();
        let realm=config.keycloak_realm.clone();
        let realm=realm.as_str();

        get_keycloak_keys( url,realm).await;
    }
    #[tokio::test(flavor ="multi_thread", worker_threads = 3)]
    async fn get_temp_access_token_test() {
        init_log();
        let url=CONTEXT.config.keycloak_auth_server_url.clone();
        let  config = ApplicationConfig::default();

        get_temp_access_token(&config.keycloak_auth_server_url,&config.keycloak_realm,&config.keycloak_resource,&config.keycloak_credentials_secret).await;
    }

    // #[tokio::test]
    // async fn reqwest_test()-> Keys {
    //     // init_log();
    //     let c = format!(
    //         "{}realms/{}/protocol/openid-connect/certs",
    //         "http://58.20.184.66:6002/auth/", "tdcare"
    //     );
    //     let body=reqwest::get(c.clone()).await.unwrap().text().await.unwrap();
    //     log::info!("{}开始访问body:{:?}",c.clone(),body);
    //
    //     let keys: Keys = reqwest::get(c.clone()).await.unwrap().json().await.unwrap();
    //
    //     log::info!("开始访问keycloak:{:?}",keys);
    //     return keys;
    //
    // }

}