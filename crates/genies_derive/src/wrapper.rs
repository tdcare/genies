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

/// impl the wrapper macro
pub(crate) fn impl_wrapper(target_fn: &ItemFn, _args: &AttributeArgs) -> TokenStream {
    let return_ty = find_return_type(target_fn);
    let func_name_ident = target_fn.sig.ident.to_token_stream();
    // let func_name = func_name_ident.to_string();

    let attrs = target_fn.attrs.clone();
    let feign_http_macro = attrs.first().unwrap().to_token_stream();

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
            "[genies] #[wrapper] 'fn {}({})' must be  async fn! ",
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
    // 因为 #[wrapper] 作用于 core-api/remote 中的 feignhttp 函数上
    // 所以这段代码在编译阶段被展开后，作用于 core-api/remote 目录中函数所在位置的mod 中。
    let wrapper_feignhttp_code = quote! {
        pub async fn #func_name_ident(#func_args) -> #return_ty{
             let bearer = genies::context::REMOTE_TOKEN.lock().unwrap().access_token.clone();
        let bearer = format!("Bearer {}", &bearer);
        let mut feignhttp_return = #feignhttp_ident( &bearer #func_args_do).await;
        return if feignhttp_return.is_ok() {
            feignhttp_return
        } else {
            let err = feignhttp_return.as_ref().err().unwrap();
            if err.to_string().contains("401 Unauthorized") {
                let remote_token_new = genies::core::jwt::get_temp_access_token(
                    &genies::context::CONTEXT.config.keycloak_auth_server_url,
                    &genies::context::CONTEXT.config.keycloak_realm,
                    &genies::context::CONTEXT.config.keycloak_resource,
                    &genies::context::CONTEXT.config.keycloak_credentials_secret,
                ).await;
                genies::context::REMOTE_TOKEN.lock().unwrap().access_token = remote_token_new.clone();
                let bearer = format!("Bearer {}", &remote_token_new);
                feignhttp_return =  #feignhttp_ident( &bearer #func_args_do).await;
            }
            feignhttp_return
        }
        }
    };

    let gen_token_temple = quote! {
         #feignhttp_code
         #wrapper_feignhttp_code
    };
    return gen_token_temple.into();
}
