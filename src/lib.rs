#[doc(hidden)]
#[macro_use]
extern crate rocket;

pub use cronframe_macro::{cron, cron_impl, cron_obj, fn_job, mt_job};
#[doc(hidden)]
pub use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
#[doc(hidden)]
pub use linkme::distributed_slice;
#[doc(hidden)]
pub use log::info;
#[doc(hidden)]
pub use once_cell::sync::Lazy;
#[doc(hidden)]
pub use std::sync::Once;
#[doc(hidden)]
pub use std::{
    any::{self, Any, TypeId},
    sync::{Arc, Mutex},
};

// lib modules
mod config;
mod cronframe;
mod cronframe_expr;
mod cronjob;
mod job_builder;
mod logger;
pub mod utils;
mod web_server;

// re-export of types
pub use config::{ConfigData, LoggerConfig, SchedulerConfig, ServerConfig};
pub use cronframe::CronFrame;
pub use cronframe::SchedulerMessage;
pub use cronframe_expr::CronFrameExpr;
pub use cronjob::{CronFilter, CronJob};
pub use job_builder::JobBuilder;

#[doc(hidden)]
pub use inventory::{collect, submit};

// necessary to gather all the global jobs automatically
collect!(JobBuilder<'static>);
