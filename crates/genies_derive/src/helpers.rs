/*
 * @Author: tzw
 * @Date: 2021-10-31 01:42:36
 * @LastEditors: tzw
 * @LastEditTime: 2021-10-31 01:42:36
 */
use syn::{Attribute, Field, Fields, FieldsNamed, PatType};
use proc_macro2::*;
use quote::quote;
use quote::ToTokens;
use syn::*;
use crate::enu::ArgType;
use crate::enu::FnArg;

pub fn find_attribute<'a>(
    name: &str,
    attributes: impl IntoIterator<Item = &'a Attribute>,
) -> Option<&'a Attribute> {
    attributes
        .into_iter()
        .find(|attr| attr.path.segments[0].ident == name)
}

pub fn find_struct_field<'a>(name: &str, fields: &'a Fields) -> Option<&'a Field> {
    match fields {
        Fields::Named(named_fields) => find_named_field(name, named_fields),
        Fields::Unnamed(_) => None,
        Fields::Unit => None,
    }
}

fn find_named_field<'a>(name: &str, fields: &'a FieldsNamed) -> Option<&'a Field> {
    fields.named.iter().find(|fld| {
        if let Some(ident) = &fld.ident {
            ident == name
        } else {
            false
        }
    })
}

pub fn list_field_idents(fields: &FieldsNamed) -> impl Iterator<Item = &Ident> {
    fields.named.iter().filter_map(|fld| fld.ident.as_ref())
}


pub fn find_return_type(target_fn: &ItemFn) -> proc_macro2::TokenStream {
    let mut return_ty = target_fn.sig.output.to_token_stream();
    match &target_fn.sig.output {
        ReturnType::Type(_, b) => {
            return_ty = b.to_token_stream();
        }
        _ => {}
    }
    let s = format!("{}", return_ty);
    if !s.contains(":: Result") && !s.starts_with("Result") {
        return_ty = quote! {
            anyhow::Result<#return_ty>
        };
    }
    return_ty
}


//find and check method return type
pub fn find_fn_body(target_fn: &ItemFn) -> proc_macro2::TokenStream {
    //del todos
    let mut target_fn = target_fn.clone();
    let mut new_stmts = vec![];
    for x in &target_fn.block.stmts {
        // println!("{:?}",x);
        let token = x.to_token_stream().to_string().replace("\n", "").replace(" ", "");
        if token.eq("todo!()") || token.eq("unimplemented!()") || token.eq("impled!()") {
            //nothing to do
        } else {
            new_stmts.push(x.to_owned());
        }
    }

    target_fn.block.stmts = new_stmts;

    target_fn.block.to_token_stream()
}





pub fn is_aggregate_ref(t:&PatType)->bool{
    let arg_name = format!("{}", t.pat.to_token_stream());
    if arg_name.contains("aggregate"){
        return true;
    }
    false
}
pub fn is_domain_event_ref(t:&PatType)->bool{
    let arg_name = format!("{}", t.pat.to_token_stream());
    if arg_name.contains("event"){
        return true;
    }
    false
}


/// Parse function args.
pub fn parse_args(sig: &mut syn::Signature) -> syn::Result<Vec<FnArg>> {
    let input = &mut sig.inputs;
    let mut req_args: Vec<FnArg> = Vec::new();
    for fn_arg in input.iter_mut() {
        if let syn::FnArg::Typed(pat_type) = fn_arg {
            let attrs = pat_type.attrs.clone();
            pat_type.attrs.clear();

            // Default is QUERY.
            let mut arg_type = ArgType::QUERY;
            let mut name = format!("{}", pat_type.pat.to_token_stream());
            let ident = syn::Ident::new(&name.clone(), proc_macro2::Span::call_site());
            match &*pat_type.ty {
                syn::Type::Path(_) | syn::Type::Reference(_) | syn::Type::Array(_) => {}
                _ => {
                    return Err(syn::Error::new_spanned(
                        &pat_type,
                        "function args type must be like `std::slice::Iter`, `&std::slice::Iter` or `[T; n]`"));
                }
            }

            if let Some(attr) = attrs.last() {
                // Content: header, param, path, body.
                let attr_ident =
                    ArgType::from_str(&attr.path.segments.last().unwrap().ident.to_string());
                if let Err(err) = attr_ident {
                    return Err(syn::Error::new_spanned(&attr.path, err));
                }
                arg_type = attr_ident.unwrap();
                // if arg_type == ArgType::HEADER && name.eq("Authorization"){
                //
                // }
                if let Some(vec) = get_metas(attr) {
                    if let Some(nested_meta) = vec.first() {
                        match nested_meta {
                            // A literal, like the `"name"` in `#[param("name")]`.
                            syn::NestedMeta::Lit(lit) => {
                                if let syn::Lit::Str(lit) = lit {
                                    if !lit.value().is_empty() {
                                        name = lit.value();
                                    }
                                }
                            },
                            _=> {
                                if let Some(name_value) = get_meta_str_value(nested_meta, "name") {
                                    name = name_value;
                                }
                            }
                        }
                    }
                }
            }

            req_args.push(FnArg {
                arg_type,
                name,
                var: ident,
                var_type: *pat_type.ty.clone(),
            });
        }
    }

    Ok(req_args)
}


pub fn get_metas(attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
    if let Ok(syn::Meta::List(mate_list)) = attr.parse_meta() {
        return Some(mate_list.nested.into_iter().collect());
    }
    None
}

pub fn get_meta_str_value(meta: &syn::NestedMeta, name: &str) -> Option<String> {
    match meta {
        // A literal, like the `"name"` in `#[param(p = "name")]`.
        syn::NestedMeta::Meta(syn::Meta::NameValue(name_value)) => {
            let key = name_value.path.segments.last().unwrap().ident.to_string();
            if key == name {
                if let syn::Lit::Str(lit) = &name_value.lit {
                    return Some(lit.value());
                }
            }
        }
        _ => {}
    }
    None
}

