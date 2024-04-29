use proc_macro::*;
use quote::quote;
use syn::{self, ItemFn};
use cron::Schedule;
use std::str::FromStr;
use std::thread;
use chrono::Utc;

#[proc_macro_attribute]
pub fn cron(att: TokenStream, code: TokenStream) -> TokenStream {        
    println!("----------------");
    println!("CRON MACRO START");
    println!("----------------");
    
    let att = att.to_string();
    let expression = &att[1..att.len()-1];

    println!("cron expression: {}", expression);
    println!("function: {}", code.to_string());

    let parsed = syn::parse::<ItemFn>(code.clone());

    if parsed.is_ok() {
        println!("Parse OK");

        let ident = parsed.clone().unwrap().sig.ident;
        let block = parsed.clone().unwrap().block;

        // aux functions identifiers
        let aux_1 = quote::format_ident!("{}_aux_1", ident);
        let aux_2 = quote::format_ident!("{}_aux_2", ident);

        let schedule = quote::format_ident!("schedule");

        let new_code = quote! {
            fn #ident() #block
            // auxiliary function for job scheduling
            fn #aux_1() {
                println!("AUX_1: job scheduling");
                let #schedule = Schedule::from_str(#expression).expect("Failed to parse CRON expression");
                loop{
                    for datetime in schedule.upcoming(Utc).take(1) {
                        let now = Utc::now();
                        let until = datetime - now;
                        thread::sleep(until.to_std().unwrap());
                        #ident();
                    }
                }
            }

            // auxiliary function for job status api
            fn #aux_2() {
                println!("AUX_2: job status api");
                #ident();
            }
        };

        println!("new_code: {}", new_code.to_string());
        println!("----------------");
        println!("CRON MACRO END");
        println!("----------------");

        return new_code.into();
    } else if let Some(error) = parsed.err() {
        println!("parse Error: {}", error);
    } else {
        unreachable!()
    }
    code
}
