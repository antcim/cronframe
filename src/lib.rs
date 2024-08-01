//! This library allows for the definition of cronjobs with macros both on functions in the "global scope" and inside struct types.
//! 
//! # General Information
//! There are three types of jobs that can be defined:
//! - global jobs
//! - functions jobs
//! - method jobs
//! 
//! Each of these is defined with a macro, a standalone macro for global jobs while function a method jobs require a little bit of setup.
//! 
//! As struct that can host jobs is known as a cron object in the context of cronframe and is defined with the cron_obj macro.
//! 
//! Jobs of a cron object must be defined inside a standalone implementation block annotated with the macro cron_impl.
//! 
//! **IMPORTANT:** a cron object must derive the Clone trait
//! 
//! The library supports a daily timeout in ms which is decativated if the value is 0.
//! 
//! # Defining A Global Job
//! ```ignore
//! #[cron(expr="* * * * * * *", timeout="0")]    
//! fn hello_job(){
//!     println!("hello world!");
//! }
//! 
//! fn main(){
//!     let cronframe = Cronframe::default();
//!     cronframe.run();
//! }
//! ```
//! 
//! # Defining A Function Job
//! ```ignore
//! #[cron_obj]
//! #[derive(Clone)] // this trait is required
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
//!     let cronframe = Cronframe::default();
//!     
//!     // this function collects all function jobs defined on a cron object
//!     User::cf_gather_fn(cronframe.clone());
//! 
//!     // start the scheduler and keep main alive
//!     cronframe.run();
//! }
//! ```
//! 
//! # Defining A Method Job
//! ```ignore
//! #[cron_obj]
//! #[derive(Clone)] // this trait is required
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
//!     fn hello_method_job(){
//!         println!("hello world!");
//!     }
//! }
//! 
//! fn main(){
//!     let cronframe = Cronframe::default();
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
//!     cronframe.run();
//! }
//! ```

#![allow(warnings)]

#[doc(hidden)]
#[macro_use]
extern crate rocket;

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

pub mod config;
pub mod cronframe;
pub mod cronjob;
pub mod job_builder;
pub mod logger;
pub mod utils;
pub mod web_server;

pub use cronframe::CronFrame;
pub use job_builder::JobBuilder;
pub use cronjob::CronJob;

pub use inventory::{collect, submit};

// necessary to gather all the global jobs automatically
collect!(JobBuilder<'static>);

/// Used in the init function of the CronJob type to account for the type of job
#[derive(Debug, Clone)]
pub enum CronJobType {
    Global(fn()),
    Method(fn(arg: Arc<Box<dyn Any + Send + Sync>>)),
    Function(fn()),
}

/// Used in the init function of the CronFrame type to filter in a single type of job for execution.
#[derive(PartialEq, Clone, Copy)]
pub enum CronFilter {
    Global,
    Function,
    Method,
}

/// This type is used in cron objects to define the cron expression and timeout for a method job.
#[derive(Debug, Clone, Default)]
pub struct CronFrameExpr {
    seconds: String,
    minutes: String,
    hour: String,
    day_month: String,
    month: String,
    day_week: String,
    year: String,
    timeout: u64,
}

impl CronFrameExpr {
    /// Creates a new CronFrameExpr instance where:
    /// - s   is seconds
    /// - m   is minutes
    /// - h   is hour
    /// - dm  is day_month
    /// - mth is month
    /// - dw  is day_week
    /// - y   is year
    /// - t   is timeout
    pub fn new(s: &str, m: &str, h: &str, dm: &str, mth: &str, dw: &str, y: &str, t: u64) -> Self {
        CronFrameExpr {
            seconds: s.to_string(),
            minutes: m.to_string(),
            hour: h.to_string(),
            day_month: dm.to_string(),
            month: mth.to_string(),
            day_week: dw.to_string(),
            year: y.to_string(),
            timeout: t,
        }
    }

    pub fn expr(&self) -> String {
        format!(
            "{} {} {} {} {} {} {}",
            self.seconds,
            self.minutes,
            self.hour,
            self.day_month,
            self.month,
            self.day_week,
            self.year
        )
    }

    pub fn timeout(&self) -> u64 {
        self.timeout
    }
}
