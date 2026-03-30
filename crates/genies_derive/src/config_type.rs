use crate::config_shared::derive_config_impl;
use crate::proc_macro::TokenStream;
use syn::DeriveInput;

const ERROR_PATH: &str = "genies::core::error::ConfigError";

pub fn derive_config_type_for_struct(ast: &DeriveInput) -> TokenStream {
    let expanded = derive_config_impl(ast, ERROR_PATH);
    TokenStream::from(expanded)
}
