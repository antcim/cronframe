use proc_macro::*;
use quote::quote;
use syn::{self, ItemFn};

#[proc_macro_attribute]
pub fn cron(att: TokenStream, code: TokenStream) -> TokenStream {        
    let att = att.to_string();
    let expression = &att[1..att.len()-1];

    println!("cron expression: {}", expression);
    println!("function: {}", code.to_string());

    let parsed = syn::parse::<ItemFn>(code.clone());

    if parsed.is_ok() {
        println!("Failed to parse job code");

        let ident = parsed.clone().unwrap().sig.ident;
        let block = parsed.clone().unwrap().block;

        // aux functions identifiers
        let aux_1 = quote::format_ident!("{}_aux_1", ident);

        let fn_name = ident.to_string();

        let new_code = quote! {
            // original function
            fn #ident() #block

            // auxiliary function for the job schedule
            fn #aux_1() -> Schedule{
                let jobname = #fn_name;
                println!("Job: {jobname} - Job Schedule");
                Schedule::from_str(#expression).expect("Failed to parse CRON expression")
            }

            inventory::submit! {
                CronJob::new(#ident, #aux_1)
            }
        };

        println!("new_code: {}", new_code.to_string());
        return new_code.into();
    } else if let Some(error) = parsed.err() {
        println!("parse Error: {}", error);
    } else {
        unreachable!()
    }
    code
}
