use proc_macro::*;
use quote::{format_ident, quote, ToTokens};
use syn::{self, parse_macro_input, punctuated::Punctuated, ItemFn, ItemImpl, ItemStruct, Meta};

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

    let mut tmp = r#struct.to_string();

    if tmp.contains("{"){
        tmp.insert_str(
            tmp.chars().count()-1,
            "tx: Option<crate::Sender<String>>",
        );
    }

    println!("tmp: {tmp}");

    let struct_edited: proc_macro2::TokenStream = tmp.parse().unwrap();

    let new_code = quote! {
        #struct_edited

        #[distributed_slice]
        static #method_jobs: [fn(Arc<Box<dyn Any + Send + Sync>>) -> JobBuilder<'static>];

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
                static #linkme_deserialize: fn(_self: Arc<Box<dyn Any + Send + Sync>>)-> JobBuilder<'static> = #impl_type::#helper;
            }
        } else {
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
            pub fn helper_gatherer(&mut self, frame: Arc<CronFrame>){
                info!("Collecting Method Jobs from {}", #type_name);
                for method_job in #method_jobs {
                    let job_builder = (method_job)(Arc::new(Box::new(self.clone())));
                    let cron_job = job_builder.build();
                    info!("Found Method Job \"{}\" from {}.", cron_job.name, #type_name);
                    self.tx = Some(cron_job.status_channels.as_ref().clone().unwrap().0.clone());
                    frame.cron_jobs.lock().unwrap().push(cron_job);
                }
                info!("Method Jobs from {} Collected.", #type_name);

                info!("Collecting Function Jobs from {}", #type_name);
                for function_job in #function_jobs {
                    let job_builder = (function_job)();
                    let cron_job = job_builder.build();
                    info!("Found Function Job \"{}\" from {}.", cron_job.name, #type_name);
                    frame.cron_jobs.lock().unwrap().push(cron_job);
                }
                info!("Method Function from {} Collected.", #type_name);
            }
        }
    };

    new_code.extend(gather_fn.into_iter());
    new_code.into()
}

#[proc_macro_attribute]
pub fn fn_job(att: TokenStream, code: TokenStream) -> TokenStream {
    let parsed = syn::parse::<ItemFn>(code.clone());

    if check_self(&parsed) {
        // self is present -> compilation error
    }

    // generate code for a function job
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

    // should contain ("timeout", "time in ms")
    let (arg_2_name, timeout) = args.peekable().nth(1).unwrap();

    if arg_1_name != "expr" && arg_2_name != "timeout" {
        // wrong argument names -> compilation error
        return code;
    }

    let origin_function = parsed.clone().unwrap().to_token_stream();
    let ident = parsed.clone().unwrap().sig.ident;
    let job_name = ident.to_string();
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

#[proc_macro_attribute]
pub fn mt_job(att: TokenStream, code: TokenStream) -> TokenStream {
    let parsed = syn::parse::<ItemFn>(code.clone());

    if !check_self(&parsed) {
        // self is missing -> compilation error
    }

    // generate code for a function job
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

    // should contain ("expr", "name of expression field")
    let (arg_1_name, cron_expr) = args.clone().peekable().nth(0).unwrap();

    if arg_1_name != "expr" {
        // wrong argument name -> compilation error
        return code;
    }

    // generate code for a method job
    let origin_method = parsed.clone().unwrap().to_token_stream();
    let ident = parsed.clone().unwrap().sig.ident;
    let job_name = ident.to_string();
    let block = parsed.clone().unwrap().block;
    let cronframe_method = format_ident!("cron_method_{}", ident);
    let helper = format_ident!("cron_helper_{}", ident);
    let expr = format_ident!("expr");
    let tout = format_ident!("tout");

    // this is to replace the native self with the self from cronframe
    let block_string = block.clone().into_token_stream().to_string();
    let mut block_string_edited = block_string.replace("self.", "cronframe_self.");
    block_string_edited.insert_str(
        1,
        "let cron_frame_instance = arg.clone();
                let cronframe_self = (*cron_frame_instance).downcast_ref::<Self>().unwrap();",
    );

    let block_edited: proc_macro2::TokenStream = block_string_edited.parse().unwrap();

    //println!("UNEDITED BLOCK:\n{block_string}");
    //println!("EDITED BLOCK:\n{block_string_edited}");

    let mut new_code = quote! {
        // original method at the user's disposal
        #origin_method

        // cronjob method at cronframe's disposal
        // fn cron_method_<name_of_method> ...
        fn #cronframe_method(arg: Arc<Box<dyn Any + Send + Sync>>) #block_edited
    };

    let helper_code = quote! {
        // fn cron_helper_<name_of_method> ...
        fn #helper(arg: Arc<Box<dyn Any + Send + Sync>>) -> JobBuilder<'static> {
            let instance = arg.clone();
            let this_obj = (*instance).downcast_ref::<Self>().unwrap();

            let #expr = this_obj.cron_expr.expr();
            let #tout = format!("{}", this_obj.cron_expr.timeout());
            let instance = arg.clone();

            JobBuilder::method_job(#job_name, Self::#cronframe_method, #expr.clone(), #tout, instance)
        }
    };

    // replace the placeholder cron_expr with the name of the field
    let helper_code_edited = helper_code.clone().into_token_stream().to_string().replace("cron_expr", &cron_expr);
    let block_edited: proc_macro2::TokenStream = helper_code_edited.parse().unwrap();

    new_code.extend(block_edited.into_iter());

    new_code.into()
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
