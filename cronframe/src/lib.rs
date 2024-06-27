#![allow(warnings)]

#[macro_use]
extern crate rocket;

pub use cronframe_macro::{cron, cron_impl, cron_obj, job};
pub use std::{
    any::{self, Any, TypeId},
    sync::{Arc, Mutex},
};
pub use linkme::distributed_slice;

pub use log::info;

mod config;
mod cronframe;
mod cronjob;
mod job_builder;
mod logger;
mod web_server;
mod utils;
mod tests_function;
mod tests_global;
mod tests_method;

pub use job_builder::JobBuilder;
pub use cronframe::CronFrame;

// necessary to gather all the annotated jobs automatically
inventory::collect!(JobBuilder<'static>);

#[derive(Debug, Clone)]
pub enum CronJobType {
    Global(fn()),
    Method(fn(arg: Arc<Box<dyn Any + Send + Sync>>)),
    Function(fn()),
}
