#[doc(hidden)]
#[macro_use] extern crate rocket;

pub use cronframe_macro::{cron, cron_impl, cron_obj, fn_job, mt_job};
#[doc(hidden)]
pub use linkme::distributed_slice;
use rocket::serde::Deserialize;
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
pub use config::{ConfigData, ServerConfig, LoggerConfig, SchedulerConfig};

#[doc(hidden)]
pub use inventory::{collect, submit};

// necessary to gather all the global jobs automatically
collect!(JobBuilder<'static>);

#[derive(Debug, PartialEq, Clone, Copy, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum CronFilter {
    None,
    Global,
    Function,
    Method,
    CLI
}


