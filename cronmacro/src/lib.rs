use std::borrow::BorrowMut;

use proc_macro::*;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{self, parse_macro_input, punctuated::Punctuated, token::Brace, ItemFn, ItemImpl, ItemStruct, Meta};

#[proc_macro_attribute]
///
/// # Cronjob Annotation Macro
///
/// This macro allows for the definition of a cronjob.
///
/// It makes use of two arguments:
///
/// - `expr = "* * * * * *"` for the cron expression.
///
/// - `timeout = "time in ms"`, 0 is for no timeout.
///
pub fn cron(att: TokenStream, code: TokenStream) -> TokenStream {
    let args = parse_macro_input!(att with Punctuated::<Meta, syn::Token![,]>::parse_terminated);

    let args = args.into_iter().map(|x| {
        x.require_name_value()
            .map(|x| {
                let arg_name = x.path.to_token_stream().to_string();
                let arg_val = x.value.to_token_stream().to_string();
                (arg_name, arg_val.replace("\"", ""))
            })
            .unwrap()
    });

    // should contain ("expr", "* * * * * *")
    let (arg_1_name, cron_expr) = args.clone().peekable().nth(0).unwrap();

    // should contain ("timeout", "i64")
    let (arg_2_name, timeout) = args.peekable().nth(1).unwrap();

    if arg_1_name == "expr" && arg_2_name == "timeout" {
        println!("cron expression: {}", cron_expr);
        println!("function: {}", code.to_string());

        let parsed = syn::parse::<ItemFn>(code.clone());

        if parsed.is_ok() {
            let ident = parsed.clone().unwrap().sig.ident;
            let block = parsed.clone().unwrap().block;

            let new_code = quote! {
                // original function
                fn #ident() #block

                // necessary for automatic job collection
                inventory::submit! {
                    JobBuilder::new(#ident, #cron_expr, #timeout)
                }
            };

            println!("new_code: {}", new_code.to_string());
            return new_code.into();
        } else if let Some(error) = parsed.err() {
            println!("parse Error: {}", error);
        } else {
            unreachable!()
        }
    }
    code
}

#[proc_macro_attribute]
pub fn cron_obj(_att: TokenStream, code: TokenStream) -> TokenStream {
    let item_struct = syn::parse::<ItemStruct>(code.clone()).unwrap();
    let r#struct = item_struct.to_token_stream();
    let cron_obj = format_ident!("CRON_OBJ_{}", item_struct.ident);

    let new_code = quote! {
        #r#struct

        lazy_static! {
            static ref #cron_obj: Mutex<Vec<CronJob>> = Mutex::new(Vec::new());
        }
    };

    println!("GENERATED CODE: {}", new_code.to_string().trim());

    new_code.into()
}

#[proc_macro_attribute]
pub fn cron_impl(_att: TokenStream, code: TokenStream) -> TokenStream {
    let item_impl = syn::parse::<ItemImpl>(code.clone()).unwrap();
    let r#impl = item_impl.to_token_stream();
    let impl_items = item_impl.items.clone();

    let mut new_code = quote! { 
        #r#impl
    };

    for item in impl_items{
        let item_token = item.to_token_stream();
        let item_fn_id = syn::parse::<ItemFn>(item_token.into()).unwrap().sig.ident;
        let impl_type = item_impl.self_ty.to_token_stream();
        let helper = format_ident!("cron_helper_{}", item_fn_id);

        let new_code_tmp = quote! { 
            inventory::submit! {
                CronObj::new(#impl_type::#helper)
            }
        };

        new_code.extend(new_code_tmp.into_iter());
    }

    println!("IMPL GEN CODE: {}", new_code.to_string().trim());

    new_code.into()
}

#[proc_macro_attribute]
pub fn job(att: TokenStream, code: TokenStream) -> TokenStream {
    let args = parse_macro_input!(att with Punctuated::<Meta, syn::Token![,]>::parse_terminated);

    let args = args.into_iter().map(|x| {
        x.require_name_value()
            .map(|x| {
                let arg_name = x.path.to_token_stream().to_string();
                let arg_val = x.value.to_token_stream().to_string();
                (arg_name, arg_val.replace("\"", ""))
            })
            .unwrap()
    });

    // should contain ("expr", "* * * * * *")
    let (arg_1_name, cron_expr) = args.clone().peekable().nth(0).unwrap();

    // should contain ("timeout", "i64")
    let (arg_2_name, timeout) = args.peekable().nth(1).unwrap();

    if arg_1_name == "expr" && arg_2_name == "timeout" {
        println!("cron expression: {}", cron_expr);
        println!("function: {}", code.to_string());

        let parsed = syn::parse::<ItemFn>(code.clone());
        

        if parsed.is_ok() {
            let ident = parsed.clone().unwrap().sig.ident;
            let block = parsed.clone().unwrap().block;

            let helper = format_ident!("cron_helper_{}", ident);

            let new_code = quote! {
                // original function
                fn #ident() #block

                fn #helper() -> JobBuilder<'static> {
                    let cronjob = JobBuilder::new(Self::#ident, #cron_expr, #timeout);
                    // let mut obj = CRON_OBJ_Users.lock().unwrap();
                    // obj.push(cronjob);
                    cronjob
                }
            };

            println!("new_code: {}", new_code.to_string());
            return new_code.into();
        } else if let Some(error) = parsed.err() {
            println!("parse Error: {}", error);
        } else {
            unreachable!()
        }
    }
    code
}