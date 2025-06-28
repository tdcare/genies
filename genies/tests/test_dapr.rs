/*
 * @Author: tzw
 * @Date: 2021-11-03 20:40:06
 * @LastEditors: tzw
 * @LastEditTime: 2021-11-29 22:25:03
 */
#[cfg(test)]
pub mod tests {
    // use salvo::test;
    // use actix_web::*;
    // use ddd_dapr::dapr::controller::*;
    //
    // #[actix_rt::test]
    // async fn test_deactivate_actor() {
    //     let mut app = test::init_service(App::new().configure(dapr_service_config)).await;
    //     let req = test::TestRequest::delete()
    //         .uri("/actors/actor1/123")
    //         .to_request();
    //     let resp = test::call_service(&mut app, req).await;
    //
    //     let status = resp.status();
    //     let result = test::read_body(resp).await;
    //
    //     println!("返回body：{:#?}", result);
    //
    //     assert!(status.is_success());
    // }
    // #[actix_rt::test]
    // pub async fn http_reqwest() {
    //     let body =
    //         reqwest::get("http://122.9.125.181/auth/realms/tdcare/protocol/openid-connect/certs")
    //             .await
    //             .unwrap();
    //
    //     println!("Response: {:?}", body);
    // }
    // #[actix_rt::test]
    // async fn test_http_awc() {
    //     //       actix_rt::System::new().block_on(async {
    //     //     let req = Client::new()
    //     //          .get("http://www.rust-lang.org")
    //     //          .header("X-TEST", "value");
    //     //         // .header(http::header::CONTENT_TYPE, "application/json");
    //     //     Ok::<_, ()>(())
    //     //  });
    //     actix_web::rt::System::new("test").block_on(async {
    //         let client = awc::Client::default();
    //         let response = client
    //             .get("http://122.9.125.181/auth/realms/tdcare/protocol/openid-connect/certs")
    //             // .header("User-Agent", "actix-web/3.0")
    //             // .timeout(Duration::from_secs(60))
    //             .send() // <- Send request
    //             .await; // <- Wait for response
    //
    //         println!("Response: {:?}", response);
    //     });
    // }
    // #[actix_rt::test]
    // async fn test_http_awc1() {
    //     let mut client = awc::Client::default();
    //     let response = client
    //         .get("http://www.rust-lang.org") // <- Create request builder
    //         .header("User-Agent", "Actix-web")
    //         .send() // <- Send http request
    //         .await;
    //
    //     println!("Response: {:?}", response);
    // }
    // #[actix_rt::test]
    // async fn test_http_hyper() {
    //     use hyper::*;
    //
    //     let client = Client::new();
    //     let res = client
    //         .get(Uri::from_static(
    //             "http://122.9.125.181/auth/realms/tdcare/protocol/openid-connect/certs",
    //         ))
    //         .await
    //         .unwrap();
    //
    //     println!("{:?}", res);
    // }
}
