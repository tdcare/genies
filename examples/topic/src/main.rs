use std::sync::Arc;
#[allow(unused_variables)] //允许未使用的变量
#[allow(dead_code)] //允许未使用的代码
#[allow(unused_must_use)]
#[allow(non_snake_case)]
use std::thread;
use genies::k8s::{k8s_health_check};

use salvo::prelude::*;
use genies::context::auth::salvo_auth;
use genies::context::CONTEXT;
use topic::{event_consumer_config};

#[tokio::main]
async fn main() {
    genies::config::log_config::init_log();


    log::info!(
        "Local: http://{}",
        CONTEXT.config.server_url.replace("0.0.0.0", "127.0.0.1") // "127.0.0.1" 替换掉 "0.0.0.0"
    );


    let _server1 = thread::spawn(|| async move {
        let app_router = Router::new()
            .push(k8s_health_check())
            .push(event_consumer_config());
           // .then(|sub_router| {
                // if CONTEXT.config.debug {
                //     sub_router.push(
                //         Router::with_path(&CONTEXT.config.servlet_path)
                //             // .hoop(salvo_auth)
                //             // .push(controller_router()),
                //     )
                // } else {
                //     // sub_router.hoop(salvo_auth).push(controller_router())
                // }
            // });
        let doc = OpenApi::new("topic", "0.0.1").merge_router(&app_router);
        let app_router = app_router
            .unshift(doc.into_router(CONTEXT.config.servlet_path.to_owned()+"/api-doc/openapi.json"))
            .unshift(SwaggerUi::new(CONTEXT.config.servlet_path.to_owned()+"/api-doc/openapi.json")
                .into_router(CONTEXT.config.servlet_path.to_owned()+"/swagger-ui"));

        let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
        Server::new(acceptor).serve(app_router).await;

    });
    _server1.join().unwrap().await
}
