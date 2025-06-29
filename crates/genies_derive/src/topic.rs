/*
 * @Author: tzw
 * @Date: 2021-11-01 00:25:38
 * @LastEditors: tzw
 * @LastEditTime: 2021-12-16 00:49:16
 */

use proc_macro2::*;
use quote::{quote, ToTokens};
// use quote::ToTokens;
use syn;
use syn::{AttributeArgs, FnArg, ItemFn};

use crate::helpers::*;
use crate::proc_macro::TokenStream;

/// impl the topic macro
pub(crate) fn impl_topic(target_fn: &ItemFn, args: &AttributeArgs) -> TokenStream {
    let return_ty = find_return_type(target_fn);
    let func_name_ident = target_fn.sig.ident.to_token_stream();
    let func_name = func_name_ident.to_string();

    let mut aggregate_ident = "".to_token_stream();
    let mut aggregate_name = String::new();
    let mut aggregate_ty_ident = "".to_token_stream();
    let mut aggregate_ty_name = String::new();
    let mut event_ident = "".to_token_stream();
    let mut event_name = String::new();
    let mut event_ty_ident = "".to_token_stream();
    let mut event_ty_name = String::new();

    for x in &target_fn.sig.inputs {
        match x {
            FnArg::Receiver(_) => {}
            FnArg::Typed(t) => {
                if is_aggregate_ref(t) {
                    aggregate_ident = t.pat.to_token_stream();
                    aggregate_name = aggregate_ident.to_string();
                    aggregate_ty_ident = t.ty.to_token_stream();
                    aggregate_ty_name = aggregate_ty_ident.to_string();
                }
                if is_domain_event_ref(t) {
                    event_ident = t.pat.to_token_stream();
                    event_name = event_ident.to_string();
                    event_ty_ident = t.ty.to_token_stream();
                    event_ty_name = event_ty_ident.to_string();
                }
            }
        }
    }

    let mut topic_name = String::new();
    let mut pubsub_name = String::new();
    let mut metadata = String::new();

    for arg in args {
        let arg_token_stream = arg.to_token_stream();
        let kv = format!("{}", arg_token_stream);
        let (k, v) = kv.split_once("=").unwrap();
        if k.contains("name") {
            topic_name = v.replace("\"", "").trim().to_string();
        }
        if k.contains("pubsub") {
            pubsub_name = v.replace("\"", "").trim().to_string();
        }
        if k.contains("metadata") {
            metadata = v.replace("\"", "").trim().to_string();
        }
    }

    let mut topic_ident = "".to_token_stream();
    if topic_name.is_empty() {
        topic_ident = quote! {
            #aggregate_ty_ident::atype().to_string()
       }
    } else {
        topic_ident = quote! {
           #topic_name.to_string()
       }
    }

    if pubsub_name.is_empty() {
        pubsub_name = "messagebus".to_string();
    }

    let mut metadata_ident = quote! {None};
    let mut metadata_hash_map = quote! {
            let mut metadata: HashMap<String, String> = HashMap::new();
    };

    if metadata.is_empty() {
        metadata_ident = quote! {None};
        metadata_hash_map = quote! {};
    } else {
        let metadata_kv: Vec<&str> = metadata.split(",").collect();
        for kv in metadata_kv {
            let temp: Vec<&str> = kv.split("=").collect();
            let k = temp[0];
            let v = temp[1];
            metadata_hash_map = quote! {
            #metadata_hash_map
            metadata.insert(#k.to_string(), #v.to_string());
        };
        }
        metadata_ident = quote! {Some(metadata)};
    }

    #[cfg(feature = "debug_mode")]
        if cfg!(debug_assertions){
            println!("{}{}{}{}{}{}{}{}{}{}", topic_ident, metadata_ident, aggregate_ident, aggregate_name, aggregate_ty_ident, aggregate_ty_name, event_ident, event_name, event_ty_ident, event_ty_name);
        }

    let func_args_stream = target_fn.sig.inputs.to_token_stream();
    let fn_body = find_fn_body(target_fn);
    let is_async = target_fn.sig.asyncness.is_some();
    if !is_async {
        panic!(
            "[genies] #[topic] 'fn {}({})' must be  async fn! ",
            func_name_ident, func_args_stream
        );
    }

    let dapr_config_wrap_fn_name = format!("{}_dapr", func_name_ident);
    let dapr_config_wrap_fn = Ident::new(&dapr_config_wrap_fn_name, Span::call_site());

    let m_struct_name = format!("{}_hoop", func_name_ident);
    let m_salvo_wrap=Ident::new(&m_struct_name, Span::call_site());

    let wrap_url = format!("/daprsub/consumers").to_lowercase();

    let dapr_config_wrap_code = quote! {
        pub  fn #dapr_config_wrap_fn() -> genies::pubsub::DaprTopicSubscription{
            use std::collections::HashMap;
            #metadata_hash_map
           let dapr_topic_subscription = genies::pubsub::DaprTopicSubscription {
                    pubsub_name: Some(#pubsub_name.to_string()),
                    topic: Some(#topic_ident),
                    route: Some(#wrap_url.to_string()),
                    routes: None,
                    metadata: #metadata_ident,
                   };
            return dapr_topic_subscription;
       }
    };

    let handler_code = quote! {
        pub async fn #func_name_ident(tx: &mut dyn rbatis::executor::Executor,#event_ident:#event_ty_ident) -> #return_ty
            #fn_body
    };

    let salvo_code=quote!{
         use salvo::prelude::*;
         #[handler]
         pub async fn #m_salvo_wrap(_req: &mut Request, _depot: &mut Depot, _res: &mut Response){
            use std::time::Duration;
            use genies::event::DomainEvent;
           // use ddd_dapr::cloud_event::CloudEvent;
            let CONSUME_STATUS_CONSUMING: String = "CONSUMING".to_string();
            let CONSUME_STATUS_CONSUMED: String = "CONSUMED".to_string();

            // let body=_req.body();
            
            let body=_req.payload().await.unwrap();

            let body=std::str::from_utf8(body).unwrap();

            log::debug!("原始:{}",body);


            let processing_expire_seconds = genies::CONTEXT.config.processing_expire_seconds as u64;
            let record_reserve_minutes = genies::CONTEXT.config.record_reserve_minutes as u64;

            let cloud_event: genies::cloud_event::CloudEvent = serde_json::from_str(&body).unwrap_or_default();
            //let cloud_event =_req.parse_json::<ddd_dapr::dapr::cloud_event::CloudEvent>().await.unwrap_or_default();

            log::debug!("{:?}",cloud_event);
            let message_imp: genies::message::MessageImpl = serde_json::from_value(cloud_event.data).unwrap_or_default();

            let payload = message_imp.payload;
            let headers = message_imp.headers;

            let event_type = headers.event_type.clone().unwrap_or_default();
            let subed_type = #event_ty_ident::default();

            // subed_type.event_type() 中这个 event_type() 是 ddd_dapr::event::DomainEvent 中的方法
            if subed_type.event_type() != event_type{
                log::debug!("不是订阅的事件类型，不进行处理。");
            }else {
                let event: #event_ty_ident = serde_json::from_str(&payload).unwrap();
                log::debug!("匹配到事件类型，事件对象为:{:?}", event);

                let mut tx = genies::CONTEXT
                                .rbatis
                                .acquire_begin()
                                .await
                                .unwrap()
                                .defer_async(|mut tx| async move{
                                    if !tx.done() {
                                        // tx.rollback().await;
                                        // log::error!("事务没有正常操作，自动进行Rollback");
                                         let r = tx.rollback().await;
                                         if let Err(e) = r {
                                               log::error!("transaction [{}] rollback fail={}", tx.tx_id, e);
                                                   } else {
                                            log::info!("transaction [{}] rollback", tx.tx_id);
                                                }
                                    }else {
                                        log::debug!("事务正常操作成功");
                                    }
                                });

                let hander_name=#func_name;
                let server_name=genies::CONTEXT.config.server_name.clone();
                let event_type_name=#event_ty_name;
                let key = format!("{}-{}-{}-{}",server_name,hander_name,event_type_name, headers.ID.clone().unwrap());
                let v = genies::CONTEXT.redis_save_service.get_string(&key).await.unwrap();
                log::debug!("当前事件redis key = {},当前事件的状态 value={:?}",key,v);
                if v.eq(&CONSUME_STATUS_CONSUMING) {
                        tx.rollback().await;
                        _depot.insert("is_retry", "true");
                        log::debug!("1事件正在处理中，事件将进行重发，key={}",key);
                    }else if v.eq(&CONSUME_STATUS_CONSUMED) {
                        tx.rollback().await;
                        log::debug!("2事件已完成,key={}",key);
                    }else {
                        let set_CONSUMING = genies::CONTEXT
                            .redis_save_service
                            .set_string_ex(
                                &key,
                                &CONSUME_STATUS_CONSUMING,
                                Some(Duration::new(processing_expire_seconds, 0)),
                            )
                            .await;

                        if set_CONSUMING.is_ok() {
                            log::debug!("3设置事件为正在消费中，开始调用事件处理程序,key={}",key);
                            let event_handle = #func_name_ident(&mut tx, event).await;
                            // 如果事件消费成功 设置redis消费 状态为 消费完成
                            if event_handle.is_ok() {
                                log::debug!("4事件处理程序处理成功,key={}",key);

                                let set_CONSUMED = genies::CONTEXT
                                    .redis_save_service
                                    .set_string_ex(
                                        &key,
                                        &CONSUME_STATUS_CONSUMED,
                                        Some(Duration::new(record_reserve_minutes * 60, 0)),
                                    )
                                    .await;

                                // redis 消费状态更新成功了，才提交数据库事务
                                if set_CONSUMED.is_ok(){
                                    tx.commit().await;
                                }else {
                                    // redis 消费状态更新失败了，数据库事务rollback，让dapr 重发消息
                                    tx.rollback().await;
                                   _depot.insert("is_retry", "true");
                                }
                            }else {
                                // 如果事件处理程序处理失败
                                tx.rollback().await;
                                genies::CONTEXT.redis_save_service.del_string(&key).await;
                                _depot.insert("is_retry", "true");
                            }
                        }else {
                            // 如果初次在 redis 中设置事件状态失败
                            log::debug!("设置redis 出错，消息重试");
                            tx.rollback().await;
                            genies::CONTEXT.redis_save_service.del_string(&key).await;
                            _depot.insert("is_retry", "true");
                        }
                    }
                
            }
        }
    };

    let gen_token_temple = quote! {
         #handler_code
         #salvo_code
         #dapr_config_wrap_code
    };
    gen_token_temple.into()
}
