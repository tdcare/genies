/*
 * @Author: tzw
 * @Date: 2021-10-31 03:05:39
 * @LastEditors: tzw
 * @LastEditTime: 2021-11-29 22:23:13
 */

 #![warn(non_snake_case)]
 use salvo::prelude::*;
 use genies_context::SERVICE_STATUS;
 use std::ops::DerefMut;
 use salvo::http::StatusCode;
 
 
 pub fn k8s_health_check()->Router {
     Router::new()
         .push(Router::with_path("/actuator/health/liveness").get(liveness))
         .push(Router::with_path("/actuator/health/readiness").get(readiness))
 }
 
 
 #[handler]
 async fn liveness(_req: &mut Request,res: &mut Response, _ctrl: &mut FlowCtrl) {
     let mut temp=SERVICE_STATUS.lock().unwrap();
     let  map=temp.deref_mut();
     let live=map.get("livenessProbe").unwrap();
     if *live {
         res.render("Ok");
     }else {
         res.status_code(StatusCode::SERVICE_UNAVAILABLE);
     }}
 
 #[handler]
 async fn readiness(_req: &mut Request,res: &mut Response, _ctrl: &mut FlowCtrl) {
     let mut temp=SERVICE_STATUS.lock().unwrap();
     let  map=temp.deref_mut();
     let live=map.get("readinessProbe").unwrap();
     if *live {
         res.render("Ok");
     }else {
         res.status_code(StatusCode::SERVICE_UNAVAILABLE);
     }
 }
 

