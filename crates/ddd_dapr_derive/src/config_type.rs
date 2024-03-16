/*
 * @Author: tzw
 * @Date: 2021-10-31 01:42:16
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-17 00:05:20
 */
use crate::helpers::find_attribute;
// use proc_macro::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{DataEnum, DataStruct, DataUnion, DeriveInput, Fields};
use proc_macro2::*;
use crate::proc_macro::TokenStream;


pub fn derive_config_type_for_struct(ast: &DeriveInput, _struct_data: &DataStruct) -> TokenStream {
    let tname = &ast.ident;
    let config_file = find_attribute("config_file", &ast.attrs)
        .map(|attr| attr.tokens.clone())
        .unwrap_or_else(|| quote!("application.yml"));
    let feilds=&_struct_data.fields;
    let  mut config_form_envs=vec![];
    match feilds {
        Fields::Named(named_fields) => {
            named_fields.named.iter().for_each(|field|{
                // println!("{},",field.ident);
                let field_name=field.ident.to_token_stream();
                let field_ty=field.ty.to_token_stream();
                let mut field_ty_string:String=field_ty.to_string();
                let mut ty_ident=quote!{};

                let mut is_option=false;
                let mut is_vec=false;
               // 如果配置项是 Option 进行特殊处理，如果是vec 不进行处理
                if field_ty_string.starts_with("Option < "){
                    is_option=true;
                    field_ty_string= field_ty_string.trim_start_matches("Option < ").parse().unwrap();
                    field_ty_string= field_ty_string.trim_end_matches(" >").parse().unwrap();
                }

                if field_ty_string.starts_with("Vec < "){
                     is_vec=true;
                     ty_ident=quote!{};
                }else {
                    let s_ident=Ident::new(&field_ty_string, Span::call_site());
                     ty_ident=quote!{#s_ident};
                }
              //  println!("{:?}",ty_ident);

                let mut config_form_env = quote! {};
                let key = field_name.to_string();
                if !is_vec {
                    if is_option {
                        config_form_env = quote! {
                                match #tname::get_env::<#ty_ident>(#key){
                                    Some(v)=>{
                                    config.#field_name=Some(v);
                                    },
                                    _=>{}
                                }
                            };
                    } else {
                        config_form_env = quote! {
                              match #tname::get_env::<#ty_ident>(#key){
                                  Some(v)=>{
                                  config.#field_name=v;
                                  },
                                  _=>{}
                              }
                          };
                    }
                    config_form_envs.push(config_form_env);
                }
            }
            )
        },
        Fields::Unnamed(_) => {},
        Fields::Unit => {},
    };

   let get_env_func=quote!{

    impl #tname  {       
       fn get_env<T:std::str::FromStr>(key:&str) ->std::option::Option<T>{
       match std::env::var(key) {
        Ok(val) => {
            match val.parse::<T>(){
                       Ok(r)=>{return Some(r)},
                       _=>None
                   }
        },
        Err(err) => {
            match std::env::var(key.to_uppercase()) {
                Ok(val) => {
                      match val.parse::<T>(){
                       Ok(r)=>{return Some(r)},
                       _=>None
                   }
                },
                Err(e) => {
                    return None;
                }
            }
        }
    }
       }
    }

   };
   // let get_config_env=quote!{
   //     let debug=get_env::<bool>("debug");
   //     let server_name=get_env::<String>("server_name");
   //     println!("{:?},{:?}",debug,server_name);
   //
   //        match get_env::<String>("server_url") {
   //          Some(v) => {
   //              println!("bbbbbbb");
   //              config.server_url = Some(v);
   //          }
   //          _ => {}
   //      }
   //
   // };

    let default=quote! {
        #[allow(unused_qualifications, unused_parens)]
        #[automatically_derived]
        impl Default for #tname  {
           fn default() -> Self{
                use std::io::Read;
                let mut f = std::fs::File::open(#config_file).unwrap();
                let mut configfile_string = String::new();
                f.read_to_string(&mut configfile_string);

                let yml_data = configfile_string;
                let mut config = serde_yaml::from_str::<#tname>(&yml_data).expect("application.yml read failed!");

                // #get_config_env
                #(#config_form_envs)*
                config
            }
            
        //    #get_env_func
        }
    };


    let code=quote!{
        #default
        #get_env_func
    };
    code.into()
}

pub fn derive_config_type_for_union(_ast: &DeriveInput, _union_data: &DataUnion) -> TokenStream {
    panic!("#[derive(Config)] is only defined for struct and enum types, but not union types")
}

pub fn derive_config_type_for_enum(_ast: &DeriveInput, _enum_data: &DataEnum) -> TokenStream {
    panic!("#[derive(Config)] is only defined for struct and enum types, but not union types")
}
