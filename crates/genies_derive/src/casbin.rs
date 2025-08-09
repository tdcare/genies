/*
 * @Author: tzw
 * @Date: 2021-11-01 00:25:38
 * @LastEditors: tzw
 * @LastEditTime: 2021-12-16 00:49:16
 */

use quote::{format_ident, quote, ToTokens};
use syn;
use syn::{parse_quote, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed};

use crate::proc_macro::TokenStream;

/// impl the casbin macro
pub(crate) fn impl_casbin(input: &mut DeriveInput) -> TokenStream {
    if let Data::Struct(DataStruct { fields: Fields::Named(FieldsNamed { named, .. }), .. }) = &mut input.data {
        // 用 parse_quote! 创建 serde 属性
        let serde_attr: syn::Attribute = parse_quote!(#[serde(skip)]);

        // enforcer 字段
        let enforcer_field = Field {
            attrs: vec![serde_attr.clone()],
            vis: syn::Visibility::Public(syn::VisPublic {
                pub_token: Default::default(),
            }),
            ident: Some(format_ident!("enforcer")),
            colon_token: Some(Default::default()),
            ty: syn::parse_str("Option<std::sync::Arc<casbin::Enforcer>>").unwrap(),
        };

        // subject 字段
        let subject_field = Field {
            attrs: vec![serde_attr],
            vis: syn::Visibility::Public(syn::VisPublic {
                pub_token: Default::default(),
            }),
            ident: Some(format_ident!("subject")),
            colon_token: Some(Default::default()),
            ty: syn::parse_str("Option<String>").unwrap(),
        };

        named.push(enforcer_field);
        named.push(subject_field);
    }
    // 获取结构体名称
    let name = &input.ident;

    // 提取字段名用于序列化
    let field_names = extract_field_names(&input);

    let expanded = quote! {
        impl #name {
            pub fn with_policy(mut self, enforcer: std::sync::Arc<casbin::Enforcer>, subject: String) -> Self {
                self.enforcer = Some(enforcer);
                self.subject = Some(subject);
                self
            }

            fn check_permission(&self, field: &str) -> bool {
                let name = salvo::oapi::naming::assign_name::<#name>(salvo::oapi::naming::NameRule::Auto);
                // log::debug!("check_permission: name={},field:{}", name,field);
                let field_str =format!("{}.{}",name,field);
                match (&self.enforcer, &self.subject) {
                    (Some(e), Some(s)) => e.enforce((s, field_str, "read")).unwrap_or(false),
                    _ => false
                }
            }
        }

        impl ::serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                use serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct(stringify!(#name), 5)?;
                #field_names
                state.end()
            }
        }
    };
    // 返回修改后的结构体定义
    TokenStream::from(quote! {
        #input
        #expanded
    })
}
fn extract_field_names(input: &DeriveInput) -> proc_macro2::TokenStream {

    match &input.data {
        Data::Struct(DataStruct { fields: Fields::Named(FieldsNamed { named, .. }), .. }) => {
            let field_checks: Vec<proc_macro2::TokenStream> = named.iter()
                .filter(|f| {
                    // 过滤掉 enforcer 和 subject 字段
                    if let Some(ident) = &f.ident {
                        ident != "enforcer" && ident != "subject"
                    } else {
                        false
                    }
                })
                .map(|f| {
                    let field_name = &f.ident;
                    let field_name_str = field_name.as_ref().unwrap().to_string();
                    // let struct_field_name_str = format!("{}.{}",name,field_name_str);
                    quote! {
                        if self.check_permission(#field_name_str) {
                            state.serialize_field(#field_name_str, &self.#field_name)?;
                        }
                    }
                })
                .collect();

            quote! {
                #(#field_checks)*
            }
        },
        _ => quote! {},
    }
}
