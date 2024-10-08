//! # CronFrame 0.1.3
//! 
//! This framework allows for the definition and scheduling of cron jobs with macros both on functions in the "global scope" and inside struct types.
//! 
//! Job creation without macros is possible, refer to the example in `no_macros.rs` on the [repo](https://github.com/antcim/cronframe).
//! 
//! # General Information
//! 
//! The cron expression parser used is [cron](https://crates.io/crates/cron).
//! 
//! Scheduling time is in UTC.
//! 
//! There are three types of jobs that can be defined using the framework:
//! - global jobs
//! - function jobs
//! - method jobs
//! 
//! Each of these is defined with a macro, a standalone macro for global jobs while function a method jobs require a little bit of setup.
//! 
//! A struct that can host jobs is known as a `cron object` in the context of cronframe and is defined with the `cron_obj` macro.
//! 
//! Jobs of a cron object must be defined inside a standalone implementation block annotated with the macro `cron_impl`.
//! 
//! **NOTICE:** a cron object derives the Clone trait so its fields must too.
//! 
//! The framework supports a daily timeout (timed-out state resets every 24hrs) in ms which is decativated if the value is 0.
//! 
//! During the first run of the framework a templates folder will be created in the current directory with 7 files inside it:
//! - base.html.tera
//! - index.htm.tera
//! - job.html.tera
//! - tingle.js
//! - cronframe.js
//! - styles.css
//! - tingle.css
//! 
//! By default the web server runs at localhost:8098, the port can be changed in the `cronframe.toml` file.
//! 
//! A rolling logger, also configurable via `cronframe.toml` provides an archive of 3 files in addition to the latest log.
//! 
//! The default size of a log file is 1MB.
//! 
//! # Tutorial
//! For the tutorial refer to the [website](https://antcim.github.io/cronframe_site/).
//! 
//! # Defining A Global Job
//! ```
//! # #[macro_use] extern crate cronframe_macro;
//! # use cronframe::CronFrame;
//! #[cron(expr="* * * * * * *", timeout="0")]    
//! fn hello_job(){
//!     println!("hello world!");
//! }
//! 
//! fn main(){
//!     // init and gather global cron jobs
//!     let cronframe = CronFrame::default();
//!     
//!     // start the scheduler
//!     cronframe.start_scheduler();
//! 
//!     // to keep the main thread alive 
//!     // cronframe.keep_alive();
//! 
//!     // alternatively, start the scheduler and keep main alive
//!     // cronframe.run();
//! }
//! ```
//! 
//! # Defining A Function Job
//! ```
//! # #[macro_use] extern crate cronframe_macro;
//! # use cronframe::CronFrame;
//! #[cron_obj]
//! struct User {
//!     name: String,
//! }
//! 
//! #[cron_impl]
//! impl User {
//!     #[fn_job(expr="* * * * * * *", timeout="0")]    
//!     fn hello_function_job(){
//!         println!("hello world!");
//!     }
//! }
//! 
//! fn main(){
//!     let cronframe = CronFrame::default();
//!     
//!     // this function collects all function jobs defined on a cron object
//!     User::cf_gather_fn(cronframe.clone());
//! 
//!     cronframe.start_scheduler();
//! 
//!     // alternatively, start the scheduler and keep main alive
//!     // cronframe.run();
//! }
//! ```
//! 
//! # Defining A Method Job
//! ```
//! # #[macro_use] extern crate cronframe_macro;
//! # use cronframe::{CronFrame, CronFrameExpr};
//! #[cron_obj]
//! struct User {
//!     name: String,
//!     expr1: CronFrameExpr,
//! }
//! 
//! #[cron_impl]
//! impl User {
//!     #[fn_job(expr="* * * * * * *", timeout="0")]    
//!     fn hello_function_job(){
//!         println!("hello world!");
//!     }
//! 
//!     #[mt_job(expr="expr1")]    
//!     fn hello_method_job(self){
//!         println!("hello world!");
//!     }
//! }
//! 
//! fn main(){
//!     let cronframe = CronFrame::default();
//! 
//!     let mut user1 = User::new_cron_obj(
//!         "John Smith".to_string(),
//!         CronFrameExpr::new("0/5", "*", "*", "*", "*", "*", "*", 0)
//!     );
//! 
//!     // this method collects all jobs defined on a cron object
//!     user1.cf_gather(cronframe.clone());
//! 
//!     // in alternative if we only wanted to collect method jobs
//!     // user1.cf_gather_mt(cronframe.clone());
//! 
//!     cronframe.start_scheduler();
//! 
//!     // alternatively, start the scheduler and keep main alive
//!     // cronframe.run();
//! }
//! ```

#[doc(hidden)]
#[macro_use] extern crate rocket;

pub use cronframe_macro::{cron, cron_impl, cron_obj, fn_job, mt_job};
#[doc(hidden)]
pub use linkme::distributed_slice;
#[doc(hidden)]
pub use std::{
    any::{self, Any, TypeId},
    sync::{Arc, Mutex},
};

#[doc(hidden)]
pub use crossbeam_channel::{bounded, unbounded, Receiver, Sender};

#[doc(hidden)]
pub use log::info;
#[doc(hidden)]
pub use once_cell::sync::Lazy;
#[doc(hidden)]
pub use std::sync::Once;

// lib modules
pub mod config;
pub mod cronframe;
pub mod cronframe_expr;
pub mod cronjob;
pub mod job_builder;
pub mod logger;
pub mod utils;
pub mod web_server;

// re-export of types
pub use cronframe::CronFrame;
pub use job_builder::JobBuilder;
pub use cronjob::CronJob;
pub use cronframe_expr::CronFrameExpr;

#[doc(hidden)]
pub use inventory::{collect, submit};

// necessary to gather all the global jobs automatically
collect!(JobBuilder<'static>);

/// Used in the init function of the CronJob type to account for the type of job
#[derive(Debug, Clone)]
pub enum CronJobType {
    Global(fn()),
    Method(fn(arg: Arc<Box<dyn Any + Send + Sync>>)),
    Function(fn()),
    CLI
}

/// Used in the init function of the CronFrame type to filter in a single type of job for execution.
/// ```
/// #[macro_use] extern crate cronframe_macro;
/// use cronframe::{CronFilter, CronFrame};
/// 
/// fn main(){
///     // allow execution of Global Jobs Only
///     let cronframe = CronFrame::init(Some(CronFilter::Global), true); 
///     // no filters for the job type
///     //let cronframe = CronFrame::init(None, true); 
/// }
/// ```
#[derive(PartialEq, Clone, Copy)]
pub enum CronFilter {
    Global,
    Function,
    Method,
    CLI
}


