use chrono::TimeDelta;
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
/// - `timeout = "time in ms"` for an optional timeout, 0 is for not timeout.
///
pub fn cron(att: TokenStream, code: TokenStream) -> TokenStream {
    let args = parse_macro_input!(att with Punctuated::<Meta, syn::Token![,]>::parse_terminated);

    // should contain "expr" 
    let arg_1 = args[0]
        .require_name_value()
        .unwrap()
        .path
        .require_ident()
        .unwrap()
        .to_string();

    // should contain "timout"
    let arg_2 = args[1]
        .require_name_value()
        .unwrap()
        .path
        .require_ident()
        .unwrap()
        .to_string();

    if arg_1 == "expr" && arg_2 == "timeout"
    {
        let cron_expr = args[0]
            .require_name_value()
            .unwrap()
            .value
            .to_token_stream()
            .to_string();

        let cron_expr = &cron_expr[1..cron_expr.len() - 1];

        let timeout = args[1]
            .require_name_value()
            .unwrap()
            .value
            .to_token_stream()
            .to_string();

        let timeout: i64 = timeout[1..timeout.len() - 1].parse().unwrap();

        println!("cron expression: {}", cron_expr);
        println!("function: {}", code.to_string());

        let parsed = syn::parse::<ItemFn>(code.clone());

        if parsed.is_ok() {
            let ident = parsed.clone().unwrap().sig.ident;
            let block = parsed.clone().unwrap().block;

            // aux functions identifiers
            let aux_1 = quote::format_ident!("{}_aux_1", ident);

            let fn_name = ident.to_string();

            let new_code = quote! {
                // original function
                fn #ident() #block

                // auxiliary function for the job schedule
                fn #aux_1() -> (Schedule, i64){
                    let jobname = #fn_name;
                    println!("Job: {jobname} - Job Schedule");
                    let schedule = Schedule::from_str(#cron_expr).expect("Failed to parse CRON expression");

                    (schedule, #timeout)
                }

                // necessary for automatic job collection
                inventory::submit! {
                    CronJob::new(#ident, #aux_1)
                }
            };

            println!("new_code: {}", new_code.to_string());
            return new_code.into();
        }else if let Some(error) = parsed.err() {
            println!("parse Error: {}", error);
        } else {
            unreachable!()
        }
    } 
    code
}
