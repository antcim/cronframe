use proc_macro::*;
use quote::{quote, ToTokens};
use syn::{self, parse_macro_input, punctuated::Punctuated, ItemFn, Meta};

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
                (arg_name, arg_val.replace("\"",""))
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
