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
use syn::{AttributeArgs, ItemFn};

use crate::helpers::*;
use crate::proc_macro::TokenStream;

/// impl the remote macro
pub(crate) fn impl_remote(target_fn: &ItemFn, _args: &AttributeArgs) -> TokenStream {
    let return_ty = find_return_type(target_fn);
    let func_name_ident = target_fn.sig.ident.to_token_stream();
    // let func_name = func_name_ident.to_string();

    let attrs = target_fn.attrs.clone();
    // Find the feignhttp HTTP method attribute (get/post/put/delete/patch),
    // skipping doc comments and other non-feignhttp attributes.
    let http_methods = ["get", "post", "put", "delete", "patch"];
    let feign_http_attr_idx = attrs.iter().position(|attr| {
        attr.path.segments.last()
            .map(|seg| http_methods.contains(&seg.ident.to_string().as_str()))
            .unwrap_or(false)
    }).expect("[genies] #[remote] requires a feignhttp HTTP method attribute (e.g., #[get(...)])");

    // Fully-qualify the feignhttp attribute path (e.g., #[get(...)] → #[feignhttp::get(...)])
    // to ensure proper resolution in the generated code regardless of user imports.
    let mut feign_http_attr = attrs[feign_http_attr_idx].clone();
    let feignhttp_segment = syn::PathSegment {
        ident: Ident::new("feignhttp", Span::call_site()),
        arguments: syn::PathArguments::None,
    };
    feign_http_attr.path.segments.insert(0, feignhttp_segment);
    let feign_http_macro = feign_http_attr.to_token_stream();

    // Collect remaining attributes (doc comments, etc.) to preserve on the generated wrapper fn
    let other_attrs: Vec<_> = attrs.iter().enumerate()
        .filter(|(i, _)| *i != feign_http_attr_idx)
        .map(|(_, a)| a)
        .collect();

    let func_args_stream = target_fn.sig.inputs.to_token_stream();

    let mut item_fn = target_fn.clone();
    let  sig = &mut item_fn.sig;
    let args = parse_args(sig).unwrap();

    let mut func_args = quote! {};
    let mut func_args_do = quote! {};
    let mut first = true;
    for arg in args {
        let temp_var = arg.var.clone();
        let temp_var_type = arg.var_type.clone();
        if first {
            func_args = quote! {
              #temp_var:#temp_var_type
          };
            first = false;
        } else {
            func_args = quote! {
          #func_args,#temp_var:#temp_var_type
      };
        }
        func_args_do = quote! {
          #func_args_do,#temp_var
      };
    }


    let fn_body = target_fn.block.to_token_stream();
    let is_async = target_fn.sig.asyncness.is_some();
    if !is_async {
        panic!(
            "[genies] #[remote] 'fn {}({})' must be  async fn! ",
            func_name_ident, func_args_stream
        );
    }

    let feignhttp_name = format!("{}_feignhttp", func_name_ident);
    let feignhttp_ident = Ident::new(&feignhttp_name, Span::call_site());

    let feignhttp_code = quote! {
        #feign_http_macro
        pub async fn #feignhttp_ident(#[header] Authorization: &str,#func_args_stream) -> #return_ty
            #fn_body
    };
    // 因为 #[remote] 作用于 core-api/remote 中的 feignhttp 函数上
    // 所以这段代码在编译阶段被展开后，作用于 core-api/remote 目录中函数所在位置的mod 中。
    let wrapper_feignhttp_code = quote! {
        #(#other_attrs)*
        pub async fn #func_name_ident(#func_args) -> #return_ty{
            // 优先使用请求级用户 token（从 salvo_auth 中间件通过 task_local 传递）
            let is_user_token = genies::context::request_token::get_request_token().is_some();
            let bearer = genies::context::request_token::get_request_token()
                .unwrap_or_else(|| {
                    // 降级：使用 service account token（用于定时任务、初始化等非请求上下文场景）
                    let token = genies::context::REMOTE_TOKEN.lock().unwrap().access_token.clone();
                    format!("Bearer {}", &token)
                });
            let mut feignhttp_return = #feignhttp_ident( &bearer #func_args_do).await;
            // 401 重试仅在使用 service account token 时触发
            if !is_user_token {
                if let Err(ref e) = feignhttp_return {
                    if e.to_string().contains("401 Unauthorized") {
                        if let Ok(remote_token_new) = genies::core::jwt::get_temp_access_token(
                            &genies::context::CONTEXT.config.keycloak_auth_server_url,
                            &genies::context::CONTEXT.config.keycloak_realm,
                            &genies::context::CONTEXT.config.keycloak_resource,
                            &genies::context::CONTEXT.config.keycloak_credentials_secret,
                        ).await {
                            genies::context::REMOTE_TOKEN.lock().unwrap().access_token = remote_token_new.clone();
                            let bearer = format!("Bearer {}", &remote_token_new);
                            feignhttp_return = #feignhttp_ident( &bearer #func_args_do).await;
                        }
                    }
                }
            }
            feignhttp_return
        }
    };

    let gen_token_temple = quote! {
         #feignhttp_code
         #wrapper_feignhttp_code
    };
    return gen_token_temple.into();
}
