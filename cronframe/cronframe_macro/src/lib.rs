use proc_macro::*;
use quote::{format_ident, quote, ToTokens};
use syn::{self, parse_macro_input, punctuated::Punctuated, spanned::Spanned, ItemFn, ItemImpl, ItemStruct, Meta};

#[proc_macro_attribute]
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

    // should contain ("timeout", "u64")
    let (arg_2_name, timeout) = args.peekable().nth(1).unwrap();

    if arg_1_name == "expr" && arg_2_name == "timeout" {
        let parsed = syn::parse::<ItemFn>(code.clone());

        if parsed.is_ok() {
            let origin_function = parsed.clone().unwrap().to_token_stream();
            let ident = parsed.clone().unwrap().sig.ident;
            let block = parsed.clone().unwrap().block;
            let job_name = ident.to_string();

            let new_code = quote! {
                // original function
                #origin_function

                // necessary for automatic job collection
                inventory::submit! {
                    JobBuilder::global_job(#job_name, #ident, #cron_expr, #timeout)
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
    let method_jobs = format_ident!("CRONFRAME_METHOD_JOBS_{}", item_struct.ident);
    let function_jobs = format_ident!("CRONFRAME_FUNCTION_JOBS_{}", item_struct.ident);

    let new_code = quote! {
        #r#struct

        #[distributed_slice]
        static #method_jobs: [fn(&dyn Any) -> JobBuilder<'static>];

        #[distributed_slice]
        static #function_jobs: [fn() -> JobBuilder<'static>];
    };

    new_code.into()
}

#[proc_macro_attribute]
pub fn cron_impl(_att: TokenStream, code: TokenStream) -> TokenStream {
    let item_impl = syn::parse::<ItemImpl>(code.clone()).unwrap();
    let r#impl = item_impl.to_token_stream();
    let impl_items = item_impl.items.clone();
    let impl_type = item_impl.self_ty.to_token_stream();

    let method_jobs = format_ident!("CRONFRAME_METHOD_JOBS_{impl_type}");
    let function_jobs = format_ident!("CRONFRAME_FUNCTION_JOBS_{impl_type}");

    let mut new_code = quote! {
        #r#impl
    };

    let mut count = 0;
    for item in impl_items {
        let item_token = item.to_token_stream();
        let item_fn_parsed = syn::parse::<ItemFn>(item_token.into());
        let item_fn_id = item_fn_parsed.clone().unwrap().sig.ident;
        let helper = format_ident!("cron_helper_{}", item_fn_id);
        let linkme_deserialize = format_ident!("LINKME_{}_{count}", item_fn_id);

        let new_code_tmp = if check_self(&item_fn_parsed) {
            // method job
            quote! {
                #[distributed_slice(#method_jobs)]
                static #linkme_deserialize: fn(_self: &dyn Any)-> JobBuilder<'static> = #impl_type::#helper;
            }
        }else{
            // function job
            quote! {
                #[distributed_slice(#function_jobs)]
                static #linkme_deserialize: fn()-> JobBuilder<'static> = #impl_type::#helper;
            }
        };

        new_code.extend(new_code_tmp.into_iter());
        count += 1;
    }

    let type_name = impl_type.to_string();

    let gather_fn = quote! {
        impl #impl_type{
            pub fn helper_gatherer(&self, frame: &mut CronFrame){
                info!("Collecting Method Jobs from {}", #type_name);
                for method_job in #method_jobs {
                    let job_builder = (method_job)(self);
                    let cron_job = job_builder.build();
                    info!("Found Method Job \"{}\" from {}.", cron_job.name, #type_name);
                    frame.cron_jobs.push(cron_job)
                }
                info!("Method Jobs from {} Collected.", #type_name);

                info!("Collecting Function Jobs from {}", #type_name);
                for method_job in #function_jobs {
                    let job_builder = (method_job)();
                    let cron_job = job_builder.build();
                    info!("Found Function Job \"{}\" from {}.", cron_job.name, #type_name);
                    frame.cron_jobs.push(cron_job)
                }
                info!("Method Function from {} Collected.", #type_name);
            }
        }
    };

    new_code.extend(gather_fn.into_iter());
    new_code.into()
}

#[proc_macro_attribute]
pub fn job(att: TokenStream, code: TokenStream) -> TokenStream {
    let parsed = syn::parse::<ItemFn>(code.clone());

    if check_self(&parsed) {
        // generate code for a method job
        let origin_method = parsed.clone().unwrap().to_token_stream();
        let ident = parsed.clone().unwrap().sig.ident;
        let job_name = ident.to_string();
        let block = parsed.clone().unwrap().block;
        let cronframe_method = format_ident!("cron_method_{}", ident);
        let helper = format_ident!("cron_helper_{}", ident);
        let expr = format_ident!("expr");
        let tout = format_ident!("tout");

        let new_code = quote! {
            // original method at the user's disposal
            #origin_method

            // cronjob method at cronframe's disposal
            // fn cron_method_<name_of_method> ...
            fn #cronframe_method(arg: &dyn Any) #block

            // fn cron_helper_<name_of_method> ...
            fn #helper(arg: &dyn Any) -> JobBuilder<'static> {
                let this_obj = Box::new(arg).downcast_ref::<Self>().unwrap();
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
                let #tout = format!("{}", this_obj.timeout);
                JobBuilder::method_job(#job_name, Self::#cronframe_method, #expr, #tout)
            }
        };
        new_code.into()
    } else {
        // generate code for a function job
        let args =
            parse_macro_input!(att with Punctuated::<Meta, syn::Token![,]>::parse_terminated);

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

        // should contain ("timeout", "u64")
        let (arg_2_name, timeout) = args.peekable().nth(1).unwrap();

        if arg_1_name != "expr" && arg_2_name != "timeout" {
            return code;
        }

        let origin_function = parsed.clone().unwrap().to_token_stream();
        let ident = parsed.clone().unwrap().sig.ident;
        let job_name = ident.to_string();
        let block = parsed.clone().unwrap().block;
        let helper = format_ident!("cron_helper_{}", ident);

        let new_code = quote! {
            // original function
            #origin_function
            
            fn #helper() -> JobBuilder<'static> {
                JobBuilder::function_job(#job_name, Self::#ident, #cron_expr, #timeout)
            }
        };
        new_code.into()
    }
}

fn check_self(parsed: &Result<ItemFn, syn::Error>) -> bool {
    if !parsed.clone().unwrap().sig.inputs.is_empty()
        && parsed
            .clone()
            .unwrap()
            .sig
            .inputs
            .first()
            .unwrap()
            .to_token_stream()
            .to_string()
            == "self"
    {
        true
    } else {
        false
    }
}
