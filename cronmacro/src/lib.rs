use proc_macro::*;
use quote::{format_ident, quote, ToTokens};
use syn::{self, parse_macro_input, punctuated::Punctuated, ItemFn, ItemImpl, ItemStruct, Meta};

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
        let parsed = syn::parse::<ItemFn>(code.clone());

        if parsed.is_ok() {
            let function = parsed.clone().unwrap().to_token_stream();
            let ident = parsed.clone().unwrap().sig.ident;
            let block = parsed.clone().unwrap().block;

            let new_code = quote! {
                // original function
                fn #ident(arg: &dyn Any) #block

                // necessary for automatic job collection
                inventory::submit! {
                    JobBuilder::new(#ident, #cron_expr, #timeout)
                }
            };

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

    new_code.into()
}

#[proc_macro_attribute]
pub fn cron_impl(_att: TokenStream, code: TokenStream) -> TokenStream {
    let item_impl = syn::parse::<ItemImpl>(code.clone()).unwrap();
    let r#impl = item_impl.to_token_stream();
    let impl_items = item_impl.items.clone();
    let impl_type = item_impl.self_ty.to_token_stream();

    let mut new_code = quote! { 
        #r#impl
    };

    let mut helper_funcs = vec![];

    for item in impl_items{
        let item_token = item.to_token_stream();
        let item_fn_id = syn::parse::<ItemFn>(item_token.into()).unwrap().sig.ident;
        let helper = format_ident!("cron_helper_{}", item_fn_id);
        helper_funcs.push(helper.clone());

        let new_code_tmp = quote! { 
            inventory::submit! {
                CronObj::new(#impl_type::#helper)
            }
        };

        new_code.extend(new_code_tmp.into_iter());
    }

    let gather_fn = quote! { 
        impl #impl_type{
            pub fn helper_gatherer(&self, frame: &mut CronFrame){
                for cron_obj in inventory::iter::<CronObj> {
                    let job_builder = (cron_obj.helper)(self);
                    frame.cron_jobs.push(job_builder.build())
                }
            }
        }
    };

    new_code.extend(gather_fn.into_iter());

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
        let parsed = syn::parse::<ItemFn>(code.clone());
        
        if parsed.is_ok() {
            let function = parsed.clone().unwrap().to_token_stream();
            let ident = parsed.clone().unwrap().sig.ident;
            let block = parsed.clone().unwrap().block;
            let self_param = parsed.clone().unwrap().sig.inputs.first().unwrap().to_token_stream().to_string();
            let helper = format_ident!("cron_helper_{}", ident);

            println!("SELF PARAM: {}", self_param);

            let expr = format_ident!("expr");

            let new_code = quote! {
                // original function
                fn #ident(arg: &dyn Any) #block

                fn #helper(arg: &dyn Any) -> Box<dyn Builder> {
                    if TypeId::of::<()>() == arg.type_id(){
                        println!("EMPTY HERE MAN");
                    }
                    else if TypeId::of::<Self>() == arg.type_id(){
                        println!("SELF HERE MAN");
                        if let Some(this_obj) = Box::new(arg).downcast_ref::<Self>() {
                            let #expr = format!(
                                "{} {} {} {} {} {} {}",
                                this_obj.second,
                                this_obj.minute,
                                this_obj.hour,
                                this_obj.day_month,
                                this_obj.month,
                                this_obj.day_week,
                                this_obj.year,
                            );

                            return Box::new(JobBuilder2::new(Self::#ident, #expr, #timeout))
                        }
                    }else{
                        println!("NO SELF HERE MAN");
                    }

                    Box::new(JobBuilder::new(Self::#ident, #cron_expr, #timeout))
                }
            };
            return new_code.into();
        } else if let Some(error) = parsed.err() {
            println!("parse Error: {}", error);
        } else {
            unreachable!()
        }
    }
    code
}