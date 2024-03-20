extern crate proc_macro;

use proc_macro::*;
use quote::quote;
use syn::{self, ItemFn};

#[proc_macro_attribute]
pub fn cron(att: TokenStream, code: TokenStream) -> TokenStream {
    println!("att: {}", att.to_string());
    println!("code: {}", code.to_string());

    let parsed = syn::parse::<ItemFn>(code.clone());

    if parsed.is_ok() {
        println!("parsed");

        let ident = parsed.clone().unwrap().sig.ident;
        let block = parsed.clone().unwrap().block;

        let aux_1 = quote::format_ident!("{}_aux_1", ident);
        let aux_2 = quote::format_ident!("{}_aux_2", ident);

        let new_code = quote! {
            fn #ident() #block

            fn #aux_1() { 
                print!("from aux_1: ");
                #ident()
            }

            fn #aux_2() {
                print!("from aux_2: ");
                #ident()
            }
        };

        println!("new_code: {new_code}");

        return new_code.into();
    } else if let Some(error) = parsed.err() {
        println!("parse Error: {}", error);
    } else {
        unreachable!()
    }
    code
}
