//! Macros for [CronFrame](https://crates.io/crates/cronframe)

use proc_macro::*;
use quote::{format_ident, quote, ToTokens};
use syn::{self, parse_macro_input, punctuated::Punctuated, ItemFn, ItemImpl, ItemStruct, Meta};

/// Global Job definition Macro
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
                cronframe::submit! {
                    cronframe::JobBuilder::global_job(#job_name, #ident, #cron_expr, #timeout)
                }
            };

            return new_code.into();
        } else if let Some(error) = parsed.err() {
            println!("parse Error: {}", error);
        }
    }
    code
}

/// Cron Object definition Macro
#[proc_macro_attribute]
pub fn cron_obj(_att: TokenStream, code: TokenStream) -> TokenStream {
    let item_struct = syn::parse::<ItemStruct>(code.clone()).unwrap();
    let r#struct = item_struct.to_token_stream();
    let ident_upper = format_ident!("{}", item_struct.ident.clone().to_string().to_uppercase());
    let struct_name = item_struct.ident;
    let method_jobs = format_ident!("CRONFRAME_METHOD_JOBS_{}", ident_upper);
    let function_jobs = format_ident!("CRONFRAME_FUNCTION_JOBS_{}", ident_upper);
    let cf_fn_jobs_flag = format_ident!("CF_FN_JOBS_FLAG_{}", ident_upper);
    let cf_fn_jobs_channels = format_ident!("CF_FN_JOBS_CHANNELS_{}", ident_upper);

    // inject the tx field for the drop of method jobs
    // this requires that the last field in the original struct is followed by a ,
    let struct_edited: proc_macro2::TokenStream = {
        let mut tmp = r#struct.to_string();
        if tmp.contains("{") {
            tmp.insert_str(
                tmp.chars().count() - 1,
                "tx: Option<cronframe::Sender<cronframe::SchedulerMessage>>",
            );
        } else {
            tmp.insert_str(
                tmp.chars().count() - 1,
                "{tx: Option<cronframe::Sender<cronframe::SchedulerMessage>>}",
            );
            tmp = (&tmp[0..tmp.len() - 1].to_string()).clone();
        }
        tmp.parse().unwrap()
    };

    // --- start --- building the new_cron_obj function
    let new_cron_obj: proc_macro2::TokenStream = {
        let type_name = struct_name.clone().into_token_stream().to_string();
        let mut function = String::from("fn new_cron_obj(");
        if !item_struct.fields.is_empty() {
            let mut tmp = item_struct.fields.iter().map(|x| {
                let field_name = x.ident.to_token_stream().to_string();
                let field_type = x.ty.to_token_stream().to_string();
                format!("{field_name} : {field_type},")
            });

            for _ in 0..item_struct.fields.len() {
                function.push_str(&tmp.next().unwrap());
            }
        }

        function.push_str(") -> ");
        function.push_str(&type_name);
        function.push_str("{");
        function.push_str(&type_name);
        function.push_str("{");

        if !item_struct.fields.is_empty() {
            let mut tmp = item_struct.fields.iter().map(|x| {
                let field_name = x.ident.to_token_stream().to_string();
                format!("{field_name},")
            });

            for _ in 0..item_struct.fields.len() {
                function.push_str(&tmp.next().unwrap());
            }
        }
        function.push_str("tx: None");
        function.push_str("}");
        function.push_str("}");
        function.parse().unwrap()
    }; // --- end --- building the new_cron_obj

    let new_code = quote! {
        // the code of the original struct with the addition of the tx field
        #[derive(Clone)]
        #struct_edited

        // used to keep track of weather function jobs have been gathered
        static #cf_fn_jobs_flag: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
        // channels used to manage to drop of function jobs
        static #cf_fn_jobs_channels: cronframe::Lazy<(cronframe::Sender<cronframe::SchedulerMessage>, cronframe::Receiver<cronframe::SchedulerMessage>)> = cronframe::Lazy::new(|| cronframe::bounded(1));

        // drop for method jobs
        impl Drop for #struct_name {
            // this drops method jobs only
            fn drop(&mut self) {
                if self.tx.is_some(){
                    let _= self.tx.as_ref().unwrap().send(cronframe::SchedulerMessage::JobDrop);
                }
            }
        }

        // drop for function jobs
        impl #struct_name {
            // the new_cron_obj function
            #new_cron_obj

            // associated funciton of cron objects to drop function jobs
            fn cf_drop_fn() {
                if *#cf_fn_jobs_flag.lock().unwrap(){
                    for func in #function_jobs{
                        let _= #cf_fn_jobs_channels.0.send(cronframe::SchedulerMessage::JobDrop);
                    }
                    *#cf_fn_jobs_flag.lock().unwrap() = false;
                }
            }
        }

        #[cronframe::distributed_slice]
        static #method_jobs: [fn(std::sync::Arc<Box<dyn std::any::Any + Send + Sync>>) -> cronframe::JobBuilder<'static>];

        #[cronframe::distributed_slice]
        static #function_jobs: [fn() -> cronframe::JobBuilder<'static>];
    };

    new_code.into()
}

/// Cron Implementation Block Macro
#[proc_macro_attribute]
pub fn cron_impl(_att: TokenStream, code: TokenStream) -> TokenStream {
    let item_impl = syn::parse::<ItemImpl>(code.clone()).unwrap();
    let r#impl = item_impl.to_token_stream();
    let impl_items = item_impl.items.clone();
    let impl_type = item_impl.self_ty.to_token_stream();

    let impl_type_upper = format_ident!(
        "{}",
        item_impl
            .self_ty
            .to_token_stream()
            .to_string()
            .to_uppercase()
    );

    let method_jobs = format_ident!("CRONFRAME_METHOD_JOBS_{impl_type_upper}");
    let function_jobs = format_ident!("CRONFRAME_FUNCTION_JOBS_{impl_type_upper}");

    let mut new_code = quote! {
        #r#impl
    };

    let mut count = 0;
    for item in impl_items {
        let item_token = item.to_token_stream();
        let item_fn_parsed = syn::parse::<ItemFn>(item_token.into());
        let item_fn_id = item_fn_parsed.clone().unwrap().sig.ident;
        let helper = format_ident!("cron_helper_{}", item_fn_id);
        let item_fn_id_upper = format_ident!(
            "{}",
            item_fn_id.to_token_stream().to_string().to_uppercase()
        );
        let linkme_deserialize = format_ident!("LINKME_{}_{count}", item_fn_id_upper);

        let new_code_tmp = if check_self(&item_fn_parsed) {
            // method job
            quote! {
                #[cronframe::distributed_slice(#method_jobs)]
                static #linkme_deserialize: fn(_self: std::sync::Arc<Box<dyn std::any::Any + Send + Sync>>)-> cronframe::JobBuilder<'static> = #impl_type::#helper;
            }
        } else {
            // function job
            quote! {
                #[cronframe::distributed_slice(#function_jobs)]
                static #linkme_deserialize: fn()-> cronframe::JobBuilder<'static> = #impl_type::#helper;
            }
        };

        new_code.extend(new_code_tmp.into_iter());
        count += 1;
    }

    let type_name = impl_type.to_string().to_uppercase();

    let cf_fn_jobs_flag = format_ident!("CF_FN_JOBS_FLAG_{}", type_name);
    let cf_fn_jobs_channels = format_ident!("CF_FN_JOBS_CHANNELS_{}", type_name);

    let gather_fn = quote! {
        impl #impl_type{
            pub fn cf_gather_mt(&mut self, frame: std::sync::Arc<CronFrame>){
                cronframe::info!("Collecting Method Jobs from {}", #type_name);
                if !#method_jobs.is_empty(){
                    let life_channels = cronframe::bounded(1);
                    self.tx = Some(life_channels.0.clone());

                    for method_job in #method_jobs {
                        let job_builder = (method_job)(std::sync::Arc::new(Box::new(self.clone())));
                        let mut cron_job = job_builder.build();
                        cron_job.add_life_channels(life_channels.clone());
                        cronframe::info!("Found Method Job \"{}\" from {}.", cron_job.name(), #type_name);
                        frame.clone().add_job(cron_job);
                    }
                    cronframe::info!("Method Jobs from {} Collected.", #type_name);
                } else {
                    cronframe::info!("Not Method Jobs from {} has been found.", #type_name);
                }
            }

            pub fn cf_gather_fn(frame: std::sync::Arc<CronFrame>){
                cronframe::info!("Collecting Function Jobs from {}", #type_name);
                if !#function_jobs.is_empty(){
                    // collect jobs from associated functions only if this is the first
                    // instance of this cron object to call the helper_gatherer function
                    let fn_flag = *#cf_fn_jobs_flag.lock().unwrap();

                    if !fn_flag {
                        for function_job in #function_jobs {
                            let job_builder = (function_job)();
                            let mut cron_job = job_builder.build();
                            cron_job.add_life_channels(#cf_fn_jobs_channels.clone());
                            cronframe::info!("Found Function Job \"{}\" from {}.", cron_job.name(), #type_name);
                            frame.clone().add_job(cron_job);
                        }
                        cronframe::info!("Function Jobs from {} Collected.", #type_name);
                        *#cf_fn_jobs_flag.lock().unwrap() = true;
                    }
                } else {
                    cronframe::info!("Not Function Jobs from {} has been found.", #type_name);
                }
            }

            pub fn cf_gather(&mut self, frame: std::sync::Arc<CronFrame>){
                Self::cf_gather_fn(frame.clone());
                self.cf_gather_mt(frame.clone());
            }
        }
    };

    new_code.extend(gather_fn.into_iter());
    new_code.into()
}

/// Function Job definition Macro for a Cron Object
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
        #[allow(dead_code)]
        #origin_function

        fn #helper() -> cronframe::JobBuilder<'static> {
            cronframe::JobBuilder::function_job(#job_name, Self::#ident, #cron_expr, #timeout)
        }
    };
    new_code.into()
}

/// Method Job definition Macro for a Cron Object
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
        #[allow(dead_code)]
        #origin_method

        // cronjob method at cronframe's disposal
        // fn cron_method_<name_of_method> ...
        fn #cronframe_method(arg: std::sync::Arc<Box<dyn std::any::Any + Send + Sync>>) #block_edited
    };

    let helper_code = quote! {
        // fn cron_helper_<name_of_method> ...
        fn #helper(arg: std::sync::Arc<Box<dyn std::any::Any + Send + Sync>>) -> cronframe::JobBuilder<'static> {
            let instance = arg.clone();
            let this_obj = (*instance).downcast_ref::<Self>().unwrap();

            let #expr = this_obj.cron_expr.expr();
            let #tout = format!("{}", this_obj.cron_expr.timeout());
            let instance = arg.clone();

            cronframe::JobBuilder::method_job(#job_name, Self::#cronframe_method, #expr.clone(), #tout, instance)
        }
    };

    // replace the placeholder cron_expr with the name of the field
    let helper_code_edited = helper_code
        .clone()
        .into_token_stream()
        .to_string()
        .replace("cron_expr", &cron_expr);
    let block_edited: proc_macro2::TokenStream = helper_code_edited.parse().unwrap();

    new_code.extend(block_edited.into_iter());

    new_code.into()
}

// aid function for fn_job and mt_job
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
