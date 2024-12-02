use proc_macro::TokenStream;
use quote::{quote, ToTokens}; 
use syn::{parse_macro_input, DeriveInput, Data, Fields, Type, Ident, Attribute};
use convert_case::{Case, Casing};
use std::str::FromStr;

/// Config derive macro for generating configuration implementations
/// 
/// This macro is part of the genies_derive crate and provides the following features:
/// - Default implementation
/// - Environment variable support with array parsing
/// - Configuration validation
/// - Configuration merging
/// - Builder pattern
/// - Hot reloading support
/// - Configuration file loading
/// - Type conversion
/// 
/// # Example
/// ```rust
/// use genies_derive::Config;
/// 
/// #[derive(Config)]
/// struct MyConfig {
///     #[config(default = "localhost")]
///     server_url: String,
///     
///     #[config(default = 1883, validate(range(min = 1, max = 65535)))]
///     port: u16,
///     
///     #[config(default = "topic1,topic2")]
///     topics: Vec<String>,
/// }
/// ```
#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let fields = match &ast.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => &fields.named,
                _ => panic!("Config derive only supports structs with named fields"),
            }
        },
        _ => panic!("Config derive only supports structs"),
    };

    // Generate field information
    let field_names: Vec<_> = fields.iter()
        .map(|f| &f.ident)
        .collect();
    let field_types: Vec<_> = fields.iter()
        .map(|f| &f.ty)
        .collect();
    let field_env_names: Vec<String> = fields.iter()
        .map(|f| f.ident.as_ref().unwrap().to_string().to_case(Case::ScreamingSnake))
        .collect();
    let field_attrs: Vec<_> = fields.iter()
        .map(|f| &f.attrs)
        .collect();

    // Generate builder field types (Option<T>)
    let builder_field_types: Vec<proc_macro2::TokenStream> = field_types.iter()
        .map(|ty| quote! { Option<#ty> })
        .collect();

    // Generate the implementations
    let expanded = quote! {
        impl #name {
            /// Validate the configuration
            pub fn validate(&self) -> Result<(), ConfigError> {
                #(
                    if let Err(e) = self.validate_field(stringify!(#field_names), &self.#field_names) {
                        return Err(e);
                    }
                )*
                Ok(())
            }

            /// Load configuration from a YAML file
            pub fn from_file(path: &str) -> Result<Self, ConfigError> {
                let contents = std::fs::read_to_string(path)
                    .map_err(|e| ConfigError::FileError(format!("Failed to read config file: {}", e)))?;
                
                serde_yaml::from_str(&contents)
                    .map_err(|e| ConfigError::ParseError(format!("Failed to parse config file: {}", e)))
            }

            /// Load configuration from multiple sources in order of priority:
            /// 1. Load from file (base configuration)
            /// 2. Override with environment variables (higher priority)
            pub fn from_sources(file_path: &str) -> Result<Self, ConfigError> {
                // First load from file
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

            /// Merge with another configuration
            pub fn merge(&mut self, other: Self) {
                #(
                    self.#field_names = other.#field_names;
                )*
            }

            /// Parse a string into a value of type T
            fn parse_value<T: FromStr>(value: &str) -> Result<T, ConfigError>
            where
                T::Err: std::fmt::Display,
            {
                value.parse()
                    .map_err(|e| ConfigError::ParseError(format!("Failed to parse value '{}': {}", value, e)))
            }

            /// Parse a comma-separated string into a Vec<T>
            fn parse_array<T: FromStr>(value: &str) -> Result<Vec<T>, ConfigError>
            where
                T::Err: std::fmt::Display,
            {
                value.split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| Self::parse_value::<T>(s))
                    .collect()
            }

            /// Check if a type is Option<T>
            fn is_option_type(type_str: &str) -> bool {
                type_str.starts_with("Option < ")
            }

            /// Extract inner type from Option<T>
            fn get_inner_type(type_str: &str) -> &str {
                if type_str.starts_with("Option < ") {
                    let start = "Option < ".len();
                    let end = type_str.len() - 2; // Remove trailing " >"
                    &type_str[start..end]
                } else {
                    type_str
                }
            }

            /// Parse a string into an Option<T>
            fn parse_option<T: FromStr>(value: &str) -> Result<Option<T>, ConfigError>
            where
                T::Err: std::fmt::Display,
            {
                if value.trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(Self::parse_value(value)?))
                }
            }

            /// Parse a string into a value based on its type
            fn parse_type(type_str: &str, value: &str) -> Result<Box<dyn std::any::Any>, ConfigError> {
                let inner_type = Self::get_inner_type(type_str);
                
                let result: Box<dyn std::any::Any> = match inner_type {
                    "String" => if Self::is_option_type(type_str) {
                        Box::new(Self::parse_option::<String>(value)?)
                    } else {
                        Box::new(Self::parse_value::<String>(value)?)
                    },
                    "i32" => if Self::is_option_type(type_str) {
                        Box::new(Self::parse_option::<i32>(value)?)
                    } else {
                        Box::new(Self::parse_value::<i32>(value)?)
                    },
                    "i64" => if Self::is_option_type(type_str) {
                        Box::new(Self::parse_option::<i64>(value)?)
                    } else {
                        Box::new(Self::parse_value::<i64>(value)?)
                    },
                    "u32" => if Self::is_option_type(type_str) {
                        Box::new(Self::parse_option::<u32>(value)?)
                    } else {
                        Box::new(Self::parse_value::<u32>(value)?)
                    },
                    "u64" => if Self::is_option_type(type_str) {
                        Box::new(Self::parse_option::<u64>(value)?)
                    } else {
                        Box::new(Self::parse_value::<u64>(value)?)
                    },
                    "f32" => if Self::is_option_type(type_str) {
                        Box::new(Self::parse_option::<f32>(value)?)
                    } else {
                        Box::new(Self::parse_value::<f32>(value)?)
                    },
                    "f64" => if Self::is_option_type(type_str) {
                        Box::new(Self::parse_option::<f64>(value)?)
                    } else {
                        Box::new(Self::parse_value::<f64>(value)?)
                    },
                    "bool" => if Self::is_option_type(type_str) {
                        Box::new(Self::parse_option::<bool>(value)?)
                    } else {
                        Box::new(Self::parse_value::<bool>(value)?)
                    },
                    _ => if type_str.starts_with("Vec < ") {
                        // Handle array types
                        let element_type = type_str.trim_start_matches("Vec < ").trim_end_matches(" >");
                        match element_type {
                            "String" => Box::new(Self::parse_array::<String>(value)?),
                            "i32" => Box::new(Self::parse_array::<i32>(value)?),
                            "i64" => Box::new(Self::parse_array::<i64>(value)?),
                            "u32" => Box::new(Self::parse_array::<u32>(value)?),
                            "u64" => Box::new(Self::parse_array::<u64>(value)?),
                            "f32" => Box::new(Self::parse_array::<f32>(value)?),
                            "f64" => Box::new(Self::parse_array::<f64>(value)?),
                            "bool" => Box::new(Self::parse_array::<bool>(value)?),
                            _ => return Err(ConfigError::ParseError(
                                format!("Unsupported array element type: {}", element_type)
                            )),
                        }
                    } else {
                        return Err(ConfigError::ParseError(
                            format!("Unsupported type: {}", type_str)
                        ));
                    }
                };
                
                Ok(result)
            }

            /// Load configuration from environment variables, overriding existing values
            pub fn load_env(&mut self) -> Result<(), ConfigError> {
                #(
                    if let Ok(value) = std::env::var(#field_env_names) {
                        let field_type = stringify!(#field_types);
                        let parsed_value = Self::parse_type(field_type, &value)?;
                        
                        // Downcast to the correct type
                        if let Some(v) = parsed_value.downcast_ref() {
                            self.#field_names = v.clone();
                        } else {
                            return Err(ConfigError::ParseError(
                                format!("Type mismatch for field {}", stringify!(#field_names))
                            ));
                        }
                    } else if Self::is_option_type(stringify!(#field_types)) {
                        // For Option types, set to None if environment variable is not present
                        self.#field_names = None;
                    }
                )*
                Ok(())
            }

            /// Create a new builder
            pub fn builder() -> #name Builder {
                #name Builder::default()
            }

            // Private validation helper
            fn validate_field<T>(&self, field_name: &str, value: &T) -> Result<(), ConfigError> {
                // Add field-specific validation based on attributes
                #(
                    if field_name == stringify!(#field_names) {
                        for attr in #field_attrs {
                            if attr.path().is_ident("config") {
                                if let Ok(meta) = attr.parse_nested_meta(|meta| {
                                    if meta.path.is_ident("validate") {
                                        // Parse validation rules
                                        meta.parse_nested_meta(|nested| {
                                            if nested.path.is_ident("range") {
                                                // Handle range validation
                                                let min = nested.value()?.parse_nested_meta(|m| {
                                                    if m.path.is_ident("min") {
                                                        Ok(m.value()?.parse()?)
                                                    } else {
                                                        Ok(std::u64::MIN)
                                                    }
                                                })?;
                                                let max = nested.value()?.parse_nested_meta(|m| {
                                                    if m.path.is_ident("max") {
                                                        Ok(m.value()?.parse()?)
                                                    } else {
                                                        Ok(std::u64::MAX)
                                                    }
                                                })?;
                                                
                                                // Perform range validation
                                                let value = value.to_string().parse::<u64>()
                                                    .map_err(|_| ConfigError::ValidationError(
                                                        format!("Invalid value for {}", field_name)
                                                    ))?;
                                                if value < min || value > max {
                                                    return Err(ConfigError::ValidationError(
                                                        format!("{} must be between {} and {}", field_name, min, max)
                                                    ));
                                                }
                                            }
                                            Ok(())
                                        })?;
                                    }
                                    Ok(())
                                }) {
                                    if let Err(e) = meta {
                                        return Err(ConfigError::ValidationError(e.to_string()));
                                    }
                                }
                            }
                        }
                    }
                )*
                Ok(())
            }

            /// Convert configuration to a different type
            pub fn convert<T: TryFrom<Self>>(&self) -> Result<T, ConfigError> {
                T::try_from(self.clone())
                    .map_err(|_| ConfigError::ConversionError(
                        format!("Failed to convert {} to target type", stringify!(#name))
                    ))
            }
        }

        /// Builder for #name
        #[derive(Default)]
        pub struct #name Builder {
            #(
                #field_names: #builder_field_types,
            )*
        }

        impl #name Builder {
            #(
                pub fn #field_names(mut self, value: #field_types) -> Self {
                    self.#field_names = Some(value);
                    self
                }
            )*

            pub fn build(self) -> Result<#name, ConfigError> {
                Ok(#name {
                    #(
                        #field_names: self.#field_names.ok_or_else(|| {
                            ConfigError::BuildError(format!("Missing required field: {}", stringify!(#field_names)))
                        })?,
                    )*
                })
            }
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(
                        #field_names: {
                            let mut default_value = None;
                            for attr in #field_attrs {
                                if attr.path().is_ident("config") {
                                    if let Ok(meta) = attr.parse_nested_meta(|meta| {
                                        if meta.path.is_ident("default") {
                                            default_value = Some(meta.value()?.parse()?);
                                        }
                                        Ok(())
                                    }) {
                                        if let Some(value) = default_value {
                                            return value;
                                        }
                                    }
                                }
                            }
                            Default::default()
                        },
                    )*
                }
            }
        }

        #[async_trait::async_trait]
        impl ConfigReload for #name {
            async fn reload(&mut self) -> Result<(), ConfigError> {
                // Try loading from file first
                if let Ok(new_config) = Self::from_file("config.yml") {
                    *self = new_config;
                }
                
                // Then override with environment variables
                self.load_env()?;
                
                // Validate after reload
                self.validate()?;
                
                Ok(())
            }
        }

        // Implement serialization traits
        impl serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct(stringify!(#name), #field_names.len())?;
                #(
                    state.serialize_field(stringify!(#field_names), &self.#field_names)?;
                )*
                state.end()
            }
        }

        impl<'de> serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::de::{self, Visitor};
                
                struct ConfigVisitor;
                
                impl<'de> Visitor<'de> for ConfigVisitor {
                    type Value = #name;
                    
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str(concat!("struct ", stringify!(#name)))
                    }
                    
                    fn visit_map<V>(self, mut map: V) -> Result<#name, V::Error>
                    where
                        V: de::MapAccess<'de>,
                    {
                        let mut config = #name::default();
                        while let Some(key) = map.next_key()? {
                            match key {
                                #(
                                    stringify!(#field_names) => {
                                        config.#field_names = map.next_value()?;
                                    }
                                )*
                                _ => {
                                    let _ = map.next_value::<de::IgnoredAny>()?;
                                }
                            }
                        }
                        Ok(config)
                    }
                }
                
                deserializer.deserialize_map(ConfigVisitor)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Error type for configuration operations
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Environment error: {0}")]
    EnvError(String),
    
    #[error("Build error: {0}")]
    BuildError(String),
    
    #[error("Reload error: {0}")]
    ReloadError(String),

    #[error("File error: {0}")]
    FileError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),
}

/// Trait for configuration hot reloading
#[async_trait::async_trait]
pub trait ConfigReload {
    async fn reload(&mut self) -> Result<(), ConfigError>;
}

/// Trait for type conversion
pub trait ConfigConvert<T> {
    fn convert(&self) -> Result<T, ConfigError>;
}
