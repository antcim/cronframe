use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn cron(att: TokenStream, code: TokenStream) -> TokenStream{
    println!("att: {}", att.to_string());
    println!("tmp: {}", code.to_string());
    
    code
}