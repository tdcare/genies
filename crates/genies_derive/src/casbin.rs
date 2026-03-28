/*
 * @Author: tzw
 * @Date: 2021-11-01 00:25:38
 * @LastEditors: tzw
 * @LastEditTime: 2021-12-16 00:49:16
 */

use quote::quote;
use syn;
use syn::{Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Type};

use crate::proc_macro::TokenStream;

/// 嵌套类型枚举
enum NestedType {
    Plain,      // 普通类型 T
    Option,     // Option<T>
    Vec,        // Vec<T>
}

/// 原始类型白名单，这些类型不需要递归过滤
const PRIMITIVE_TYPES: &[&str] = &[
    // 数值类型
    "i8", "i16", "i32", "i64", "i128",
    "u8", "u16", "u32", "u64", "u128",
    "f32", "f64",
    "isize", "usize",
    // 字符串类型
    "String", "str",
    // 布尔类型
    "bool",
    // 字符类型
    "char",
];

/// 检查类型名是否为原始类型
fn is_primitive_type(type_name: &str) -> bool {
    PRIMITIVE_TYPES.contains(&type_name)
}

/// 检测类型的包装类型（Plain、Option 还是 Vec）
fn detect_wrapper_type(ty: &Type) -> NestedType {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                return NestedType::Option;
            } else if segment.ident == "Vec" {
                return NestedType::Vec;
            }
        }
    }
    NestedType::Plain
}

/// 从类型中提取最内层类型名
/// Vec<BankAccount> → "BankAccount"
/// Option<Address> → "Address"
/// Address → "Address"
fn extract_type_name(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            // 如果是 Option 或 Vec，提取内部类型
            if segment.ident == "Option" || segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner_ty) = arg {
                            return extract_type_name(inner_ty);
                        }
                    }
                }
            }
            // 返回类型名
            return segment.ident.to_string();
        }
    }
    String::new()
}

/// impl the casbin macro
pub(crate) fn impl_casbin(input: &mut DeriveInput) -> TokenStream {
    // 获取结构体名称
    let name = &input.ident;

    // 自动检测嵌套字段：提取内层类型名，非原始类型的字段需要递归过滤
    let nested_field_info: Vec<(String, NestedType, String)> = if let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = &input.data
    {
        named
            .iter()
            .filter_map(|f| {
                let inner_type_name = extract_type_name(&f.ty);
                // 只有非原始类型才需要递归过滤
                if !inner_type_name.is_empty() && !is_primitive_type(&inner_type_name) {
                    f.ident.as_ref().map(|ident| {
                        let field_name = ident.to_string();
                        let wrapper_type = detect_wrapper_type(&f.ty);
                        (field_name, wrapper_type, inner_type_name)
                    })
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    };

    // 移除字段上的 #[casbin(...)] 属性（防止编译器报未知属性错误）
    if let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = &mut input.data
    {
        for field in named.iter_mut() {
            field.attrs.retain(|attr| {
                !(attr.path.segments.len() == 1 && attr.path.segments[0].ident == "casbin")
            });
        }
    }

    // 生成 nested 字段的递归过滤代码
    let nested_filter_code: Vec<proc_macro2::TokenStream> = nested_field_info
        .iter()
        .map(|(field_name, wrapper_type, type_name)| {
            match wrapper_type {
                NestedType::Plain => {
                    // 普通类型 T
                    quote! {
                        if let Some(v) = map.get_mut(#field_name) {
                            genies_auth::casbin_filter_object(v, #type_name, enforcer, subject);
                        }
                    }
                }
                NestedType::Option => {
                    // Option<T> 类型
                    quote! {
                        if let Some(v) = map.get_mut(#field_name) {
                            if !v.is_null() {
                                genies_auth::casbin_filter_object(v, #type_name, enforcer, subject);
                            }
                        }
                    }
                }
                NestedType::Vec => {
                    // Vec<T> 类型
                    quote! {
                        if let Some(serde_json::Value::Array(arr)) = map.get_mut(#field_name) {
                            for item in arr.iter_mut() {
                                genies_auth::casbin_filter_object(item, #type_name, enforcer, subject);
                            }
                        }
                    }
                }
            }
        })
        .collect();

    let expanded = quote! {
        impl #name {
            /// 对 JSON Value 树进行递归权限过滤
            pub fn casbin_filter(
                value: &mut serde_json::Value,
                enforcer: &casbin::Enforcer,
                subject: &str,
            ) {
                use casbin::CoreApi;
                let type_name = salvo::oapi::naming::assign_name::<#name>(salvo::oapi::naming::NameRule::Auto);

                if let serde_json::Value::Object(map) = value {
                    // 1. 过滤自身字段
                    let keys: Vec<String> = map.keys().cloned().collect();
                    for key in keys {
                        let resource = format!("{}.{}", type_name, key);
                        match enforcer.enforce((subject, &resource, "read")) {
                            Ok(false) => { map.remove(&key); }
                            _ => {}  // Ok(true) 或 Err 都保留字段
                        }
                    }

                    // 2. 递归过滤嵌套字段
                    #(#nested_filter_code)*
                }
            }
        }
    };

    // 生成 Writer trait 实现
    let writer_impl = quote! {
        #[async_trait::async_trait]
        impl salvo::writing::Writer for #name {
            async fn write(mut self, _req: &mut salvo::prelude::Request, depot: &mut salvo::prelude::Depot, res: &mut salvo::prelude::Response) {
                // 1. 提取权限信息
                let enforcer = depot.get::<std::sync::Arc<casbin::Enforcer>>("casbin_enforcer").ok().cloned();
                let subject = depot.get::<String>("casbin_subject").ok().cloned();

                // 2. 标准序列化为 JSON Value
                match serde_json::to_value(&self) {
                    Ok(mut value) => {
                        // 3. 递归权限过滤
                        if let (Some(ref e), Some(ref s)) = (enforcer, subject) {
                            Self::casbin_filter(&mut value, e, s);
                        }
                        // 4. 写入已过滤的响应
                        res.render(salvo::prelude::Json(value));
                    }
                    Err(e) => {
                        res.status_code(salvo::http::StatusCode::INTERNAL_SERVER_ERROR);
                        res.render(format!("Serialization error: {}", e));
                    }
                }
            }
        }
    };

    // 返回修改后的结构体定义
    TokenStream::from(quote! {
        #input
        #expanded
        #writer_impl
    })
}
