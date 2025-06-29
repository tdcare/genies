
extern crate proc_macro;


mod aggregate_type;
mod event_type;
mod config_type;
mod helpers;
mod topic;
mod wrapper;
mod enu;


use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Data, Fields, DataStruct, Attribute, Type};

use crate::topic::*;
use syn::*;

use crate::aggregate_type::{
    derive_aggregate_type_for_enum, derive_aggregate_type_for_struct,
    derive_aggregate_type_for_union,
};
use crate::event_type::{
    derive_event_type_for_enum, derive_event_type_for_struct, derive_event_type_for_union,
};

use crate::config_type::{derive_config_type_for_struct};
use crate::wrapper::impl_wrapper;
/// 对领域事件进行标记
#[proc_macro_derive(DomainEvent, attributes(event_type, event_type_version, event_source))]
pub fn derive_event_type(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    match ast.data {
        Data::Enum(ref enum_data) => derive_event_type_for_enum(&ast, enum_data),
        Data::Struct(ref struct_data) => derive_event_type_for_struct(&ast, struct_data),
        Data::Union(ref union_data) => derive_event_type_for_union(&ast, union_data),
    }
}
/// 对聚合根进行标记
#[proc_macro_derive(
    Aggregate,
    attributes(aggregate_type, id_field, initialize_with_defaults)
)]
pub fn derive_aggregate_type(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match ast.data {
        Data::Enum(ref enum_data) => derive_aggregate_type_for_enum(&ast, enum_data),
        Data::Struct(ref struct_data) => derive_aggregate_type_for_struct(&ast, struct_data),
        Data::Union(ref union_data) => derive_aggregate_type_for_union(&ast, union_data),
    }
}


/// 事件消费宏
#[proc_macro_attribute]
pub fn topic(args: TokenStream, func: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let target_fn: ItemFn = syn::parse(func).unwrap();
    let stream = impl_topic(&target_fn, &args);
    #[cfg(feature = "debug_mode")]
        if cfg!(debug_assertions){
            use rust_format::{Formatter, RustFmt};
            let code = RustFmt::default().format_str(stream.to_string()).unwrap();
            println!(
                "............gen macro topic :\n {}",
                code
            );
            println!("............gen macro topic end............");
        }

    return stream;
}

/// 对feignhttp 请求进行包装，自动获取jwt token。当jwt token 失效时，会自动更新
#[proc_macro_attribute]
pub fn wrapper(args: TokenStream, func: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let target_fn: ItemFn = syn::parse(func).unwrap();
    let stream = impl_wrapper(&target_fn, &args);
    #[cfg(feature = "debug_mode")]
       if cfg!(debug_assertions) {
            use rust_format::{Formatter, RustFmt};
            let code = RustFmt::default().format_str(stream.to_string()).unwrap();
            println!(
                "............gen macro wrapper :\n {}",
                code
            );
            println!("............gen macro wrapper end............");
        }

    return stream;
}



#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
  

    let stream=  derive_config_type_for_struct(&ast);
 
    #[cfg(feature = "debug_mode")]
        if cfg!(debug_assertions){
            use rust_format::{Formatter, RustFmt};
            let code = RustFmt::default().format_str(stream.to_string()).unwrap();
            println!(
                "............gen macro Config :\n {}",
                code
            );
            println!("............gen macro Config end............");
        }

    return stream;



}

