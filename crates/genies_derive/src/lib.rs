use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, DataStruct, Attribute, Type};
use convert_case::{Case, Casing};

#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let fields = if let Data::Struct(DataStruct { fields: Fields::Named(ref fields), .. }) = ast.data {
        &fields.named
    } else {
        panic!("Config can only be derived for structs with named fields");
    };

    let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_env_names: Vec<String> = fields.iter()
        .map(|f| f.ident.as_ref().unwrap().to_string().to_case(Case::ScreamingSnake))
        .collect();

    // 处理每个字段的默认值
    let default_values: Vec<proc_macro2::TokenStream> = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;
        
        // 查找 #[config(default = "...")] 属性
        let mut has_default_attr = false;
        let mut default_value = None;
        
        for attr in &field.attrs {
            if attr.path().is_ident("config") {
                if let Ok(meta) = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("default") {
                        has_default_attr = true;
                        if let Ok(lit) = meta.value()?.parse::<syn::LitStr>() {
                            default_value = Some(lit.value());
                        }
                    }
                    Ok(())
                }) {
                    // 属性解析成功
                }
            }
        }
        
        // 根据字段类型生成默认值
        let default_expr = if has_default_attr && default_value.is_some() {
            let value = default_value.unwrap();
            generate_default_value(field_type, &value)
        } else {
            // 没有默认值属性，使用 Default::default()
            quote! { Default::default() }
        };
        
        quote! {
            #field_name: #default_expr
        }
    }).collect();

    let expanded = quote! {
        impl #name {
            pub fn validate(&self) -> Result<(), genies::error::ConfigError> {
                Ok(())
            }

            pub fn from_file(path: &str) -> Result<Self, genies::error::ConfigError> {
                let contents = std::fs::read_to_string(path)
                    .map_err(|e| genies::error::ConfigError::FileError(format!("Failed to read config file: {}", e)))?;
                serde_yaml::from_str(&contents)
                    .map_err(|e| genies::error::ConfigError::ParseError(format!("Failed to parse config file: {}", e)))
            }

            pub fn from_sources(file_path: &str) -> Result<Self, genies::error::ConfigError> {
                // First try to load from file
                let mut config = match Self::from_file(file_path) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        log::warn!("Failed to load config from file: {}, using defaults", e);
                        Self::default()
                    }
                };

                // Then override with environment variables
                if let Err(e) = config.load_env() {
                    log::warn!("Failed to load config from environment: {}", e);
                }

                // Validate the final configuration
                config.validate()?;

                Ok(config)
            }

            pub fn merge(&mut self, other: Self) {
                #(self.#field_names = other.#field_names;)*
            }

            pub fn load_env(&mut self) -> Result<(), genies::error::ConfigError> {
                #(if let Ok(value) = std::env::var(#field_env_names) {
                    log::info!("Found environment variable {}: {}", #field_env_names, value);
                })*
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

// 根据字段类型生成默认值
fn generate_default_value(ty: &Type, value: &str) -> proc_macro2::TokenStream {
    if is_option_type(ty) {
        // Option<T> 类型
        let inner_type = get_option_inner_type(ty);
        let inner_default = generate_default_value(inner_type, value);
        quote! { Some(#inner_default) }
    } else if is_vec_type(ty) {
        // Vec<T> 类型
        let inner_type = get_vec_inner_type(ty);
        if value.is_empty() {
            quote! { Vec::new() }
                        } else {
            // 解析逗号分隔的值
            let values: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
            let inner_defaults: Vec<proc_macro2::TokenStream> = values.iter()
                .map(|v| generate_default_value(inner_type, v))
                .collect();
            quote! { vec![#(#inner_defaults),*] }
        }
                                                    } else {
        // 基本类型
        match get_basic_type(ty) {
            "String" => quote! { #value.to_string() },
            "u16" => quote! { #value.parse::<u16>().unwrap_or(0) },
            "u32" => quote! { #value.parse::<u32>().unwrap_or(0) },
            "u64" => quote! { #value.parse::<u64>().unwrap_or(0) },
            "i16" => quote! { #value.parse::<i16>().unwrap_or(0) },
            "i32" => quote! { #value.parse::<i32>().unwrap_or(0) },
            "i64" => quote! { #value.parse::<i64>().unwrap_or(0) },
            "f32" => quote! { #value.parse::<f32>().unwrap_or(0.0) },
            "f64" => quote! { #value.parse::<f64>().unwrap_or(0.0) },
            "bool" => quote! { #value.parse::<bool>().unwrap_or(false) },
            _ => quote! { #value.to_string() }
        }
    }
}

// 辅助函数：检查是否为 Option 类型
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            segment.ident == "Option"
        } else {
            false
        }
    } else {
        false
    }
}

// 辅助函数：检查是否为 Vec 类型
fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            segment.ident == "Vec"
        } else {
            false
        }
    } else {
        false
    }
}

// 获取 Option 的内部类型
fn get_option_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                    return inner_ty;
                }
            }
        }
    }
    ty
}

// 获取 Vec 的内部类型
fn get_vec_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                    return inner_ty;
                }
            }
        }
    }
    ty
}

// 获取基本类型名称
fn get_basic_type(ty: &Type) -> &'static str {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            match segment.ident.to_string().as_str() {
                "String" => "String",
                "u16" => "u16",
                "u32" => "u32",
                "u64" => "u64",
                "i16" => "i16",
                "i32" => "i32",
                "i64" => "i64",
                "f32" => "f32",
                "f64" => "f64",
                "bool" => "bool",
                _ => "String"
            }
        } else {
            "String"
        }
    } else {
        "String"
    }
}