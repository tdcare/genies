use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, DataStruct};
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

    let builder_field_types: Vec<proc_macro2::TokenStream> = field_types.iter()
        .map(|ty| quote! { Option<#ty> })
        .collect();

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
                let mut config = match Self::from_file(file_path) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        log::warn!("Failed to load config from file: {}, using defaults", e);
                        Self::default()
                    }
                };
                if let Err(e) = config.load_env() {
                    log::warn!("Failed to load config from environment: {}", e);
                }
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
                Self { #(#field_names: Default::default(),)* }
            }
        }
    };

    TokenStream::from(expanded)
}