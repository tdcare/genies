use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields, Type};
use proc_macro2::*;
use crate::proc_macro::TokenStream;
use convert_case::{Case, Casing};

pub fn derive_config_type_for_struct(ast: &DeriveInput) -> TokenStream {

    let name = &ast.ident;

    let fields = if let Data::Struct(DataStruct { fields: Fields::Named(ref fields), .. }) = ast.data {
        &fields.named
    } else {
        panic!("Config can only be derived for structs with named fields");
    };

    let mut default_values = Vec::new();
    let mut env_parse_code = Vec::new();
    let mut merge_code = Vec::new();

    for field in fields.iter() {
        let field_name = &field.ident;
        let field_type = &field.ty;
        let env_name = field_name.as_ref().unwrap().to_string().to_case(Case::ScreamingSnake);

        // 解析 #[config(default = "...")]
        let mut default_value = quote! { Default::default() };
        for attr in &field.attrs {
            if attr.path.is_ident("config") {
                if let Ok(meta) = attr.parse_meta() {
                    if let syn::Meta::List(meta_list) = meta {
                        for nested_meta in meta_list.nested {
                            if let syn::NestedMeta::Meta(syn::Meta::NameValue(name_value)) = nested_meta {
                                if name_value.path.is_ident("default") {
                                    if let syn::Lit::Str(lit_str) = &name_value.lit {
                                        let val = lit_str.value();
                                        default_value = generate_default_value(field_type, &val);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        default_values.push(quote! { #field_name: #default_value });

        // 生成环境变量解析代码
        env_parse_code.push(generate_env_parse_code(field_name, field_type, &env_name));
        // 生成 merge 代码
        merge_code.push(quote! { self.#field_name = other.#field_name; });
    }

    let expanded = quote! {
        impl #name {
            pub fn validate(&self) -> Result<(), genies_core::error::ConfigError> {
                Ok(())
            }

            pub fn from_file(path: &str) -> Result<Self, genies_core::error::ConfigError> {
                let contents = std::fs::read_to_string(path)
                    .map_err(|e| genies_core::error::ConfigError::FileError(format!("Failed to read config file: {}", e)))?;
                serde_yaml::from_str(&contents)
                    .map_err(|e| genies_core::error::ConfigError::ParseError(format!("Failed to parse config file: {}", e)))
            }

            pub fn from_sources(file_path: &str) -> Result<Self, genies_core::error::ConfigError> {
                let mut config = Self::default();
                if let Ok(contents) = std::fs::read_to_string(file_path) {
                    match serde_yaml::from_str::<Self>(&contents) {
                        Ok(file_config) => {
                            config.merge(file_config);
                            log::info!("Loaded config from file: {}", file_path);
                        }
                    Err(e) => {
                            log::warn!("Failed to parse config file {}: {}, using defaults", file_path, e);
                        }
                    }
                } else {
                    log::warn!("Config file not found: {}, using defaults", file_path);
                }
                config.load_env()?;
                config.validate()?;
                Ok(config)
            }

            pub fn merge(&mut self, other: Self) {
                #(#merge_code)*
            }

            pub fn load_env(&mut self) -> Result<(), genies_core::error::ConfigError> {
                #(#env_parse_code)*
                Ok(())
            }
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(#default_values,)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

// 生成默认值代码
fn generate_default_value(ty: &Type, value: &str) -> proc_macro2::TokenStream {
    if is_option_type(ty) {
        let inner = get_option_inner_type(ty);
        let inner_default = generate_default_value(inner, value);
        quote! { Some(#inner_default) }
    } else if is_vec_type(ty) {
        let inner = get_vec_inner_type(ty);
        if value.trim().is_empty() {
            quote! { Vec::new() }
                        } else {
            let values: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
            let inner_defaults: Vec<_> = values.iter().map(|v| generate_default_value(inner, v)).collect();
            quote! { vec![#(#inner_defaults),*] }
        }
    } else {
        match get_basic_type(ty) {
            "String" => quote! { #value.to_string() },
            "bool" => quote! { #value.parse::<bool>().unwrap_or(false) },
            "u8" => quote! { #value.parse::<u8>().unwrap_or(0) },
            "u16" => quote! { #value.parse::<u16>().unwrap_or(0) },
            "u32" => quote! { #value.parse::<u32>().unwrap_or(0) },
            "u64" => quote! { #value.parse::<u64>().unwrap_or(0) },
            "u128" => quote! { #value.parse::<u128>().unwrap_or(0) },
            "i8" => quote! { #value.parse::<i8>().unwrap_or(0) },
            "i16" => quote! { #value.parse::<i16>().unwrap_or(0) },
            "i32" => quote! { #value.parse::<i32>().unwrap_or(0) },
            "i64" => quote! { #value.parse::<i64>().unwrap_or(0) },
            "i128" => quote! { #value.parse::<i128>().unwrap_or(0) },
            "f32" => quote! { #value.parse::<f32>().unwrap_or(0.0) },
            "f64" => quote! { #value.parse::<f64>().unwrap_or(0.0) },
            _ => quote! { #value.to_string() }
        }
    }
}

// 生成环境变量解析代码
fn generate_env_parse_code(field_name: &Option<syn::Ident>, ty: &Type, env_name: &str) -> proc_macro2::TokenStream {
    let field = field_name.as_ref().unwrap();
    if is_option_vec_type(ty) {
        let inner = get_option_vec_inner_type(ty);
        quote! {
            if let Ok(value) = std::env::var(#env_name) {
                self.#field = if value.trim().is_empty() {
                    None
                                                    } else {
                    Some(
                        value.split(',')
                            .map(|s| s.trim().parse::<#inner>().map_err(|e| genies_core::error::ConfigError::ParseError(format!("Failed to parse Option<Vec>: {}", e))))
                            .collect::<Result<Vec<#inner>, _>>()?
                    )
                };
            }
        }
    } else if is_option_type(ty) {
        let inner = get_option_inner_type(ty);
        quote! {
            if let Ok(value) = std::env::var(#env_name) {
                self.#field = if value.trim().is_empty() {
                    None
                                                    } else {
                    Some(value.parse::<#inner>().map_err(|e| genies_core::error::ConfigError::ParseError(format!("Failed to parse Option: {}", e)))?)
                };
            }
        }
    } else if is_vec_type(ty) {
        let inner = get_vec_inner_type(ty);
        quote! {
            if let Ok(value) = std::env::var(#env_name) {
                self.#field = if value.trim().is_empty() {
                    Vec::new()
                } else {
                    value.split(',')
                        .map(|s| s.trim().parse::<#inner>().map_err(|e| genies_core::error::ConfigError::ParseError(format!("Failed to parse Vec: {}", e))))
                        .collect::<Result<Vec<#inner>, _>>()?
                };
            }
        }
    } else {
        quote! {
            if let Ok(value) = std::env::var(#env_name) {
                self.#field = value.parse().map_err(|e| genies_core::error::ConfigError::ParseError(format!("Failed to parse: {}", e)))?;
            }
        }
    }
}

// 类型辅助函数
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().map_or(false, |seg| seg.ident == "Option")
    } else { false }
}
fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().map_or(false, |seg| seg.ident == "Vec")
    } else { false }
}
fn is_option_vec_type(ty: &Type) -> bool {
    if is_option_type(ty) {
        let inner = get_option_inner_type(ty);
        is_vec_type(inner)
    } else { false }
}
fn get_option_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                    return inner_ty;
                }
            }
        }
    }
    ty
}
fn get_vec_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                    return inner_ty;
                }
            }
        }
    }
    ty
}
fn get_option_vec_inner_type(ty: &Type) -> &Type {
    get_vec_inner_type(get_option_inner_type(ty))
}
fn get_basic_type(ty: &Type) -> &'static str {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            match seg.ident.to_string().as_str() {
                "String" => "String",
                "bool" => "bool",
                "u8" => "u8",
                "u16" => "u16",
                "u32" => "u32",
                "u64" => "u64",
                "u128" => "u128",
                "i8" => "i8",
                "i16" => "i16",
                "i32" => "i32",
                "i64" => "i64",
                "i128" => "i128",
                "f32" => "f32",
                "f64" => "f64",
                _ => "String"
            }
        } else { "String" }
    } else { "String" }
}