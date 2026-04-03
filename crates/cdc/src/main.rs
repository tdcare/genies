use salvo::prelude::*;
use rbatis::RBatis;
use std::ops::DerefMut;
use log::*;
use salvo::http::StatusCode;

use cdc::config::{CONTEXT, POOLS, SERVICE_STATUS};
use cdc::config::log::init_log;
use cdc::dapr_message_box::dapr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_log(&CONTEXT.config);

    // Spawn CDC polling tasks
    for (service_name, cdc_config) in &CONTEXT.config.cdc_configs {
        let pool = POOLS.get(&service_name.clone());
        if pool.is_none() {
            warn!("No database pool found for service: {}, skipping", service_name);
            continue;
        }
        let rb = pool.unwrap().clone();
        let addr = CONTEXT.config.dapr_http.clone();
        let dapr_pubsub_name = cdc_config.dapr_pubsub_name.as_ref().unwrap().clone();
        let dapr_pub_message_limit: i64 = cdc_config.dapr_pub_message_limit.unwrap();
        let dapr_cdc_message_period = cdc_config.dapr_cdc_message_period.unwrap();
        let clear_message_before_second = cdc_config.clear_message_before_second.unwrap();
        let svc_name = service_name.clone();

        tokio::spawn(async move {
            loop {
                match async_template(
                    rb.clone(),
                    addr.clone(),
                    dapr_pubsub_name.clone(),
                    dapr_pub_message_limit,
                    clear_message_before_second,
                    &svc_name,
                ).await {
                    Ok(_) => (),
                    Err(e) => error!("CDC task failed for service: {}, error: {}", svc_name, e),
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    dapr_cdc_message_period as u64,
                )).await;
            }
        });
    }

    // Run HTTP health check server on main task
    let router = Router::new()
        .push(Router::with_path("/actuator/health/liveness").get(liveness))
        .push(Router::with_path("/actuator/health/readiness").get(readiness));
    let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
    Server::new(acceptor).serve(router).await;

    Ok(())
}

pub async fn async_template(
    rb: RBatis,
    addr: String,
    dapr_pubsub_name: String,
    dapr_pub_message_limit: i64,
    clear_message_before_second: i64,
    service_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let message_box = dapr(rb, addr, dapr_pubsub_name, dapr_pub_message_limit, clear_message_before_second).await;
    if message_box.is_err() {
        error!("Message processing failed for service: {}, setting liveness to false", service_name);
        let mut temp = SERVICE_STATUS.lock().unwrap();
        let map = temp.deref_mut();
        map.insert("livenessProbe".to_string(), false);
    }
    Ok(())
}

#[handler]
async fn liveness(_req: &mut Request, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let mut temp = SERVICE_STATUS.lock().unwrap();
    let map = temp.deref_mut();
    let live = map.get("livenessProbe").unwrap();
    if *live {
        res.render("Ok");
    } else {
        res.status_code(StatusCode::SERVICE_UNAVAILABLE);
    }
}

#[handler]
async fn readiness(_req: &mut Request, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let mut temp = SERVICE_STATUS.lock().unwrap();
    let map = temp.deref_mut();
    let live = map.get("readinessProbe").unwrap();
    if *live {
        res.render("Ok");
    } else {
        res.status_code(StatusCode::SERVICE_UNAVAILABLE);
    }
}
