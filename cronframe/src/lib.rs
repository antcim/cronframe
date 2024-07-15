#![allow(warnings)]

#[macro_use]
extern crate rocket;

pub use cronframe_macro::{cron, cron_impl, cron_obj, fn_job, mt_job};
pub use linkme::distributed_slice;
pub use std::{
    any::{self, Any, TypeId},
    sync::{Arc, Mutex},
};

pub use crossbeam_channel::{bounded, Sender, Receiver};

pub use once_cell::sync::Lazy;
pub use std::sync::Once;
pub use log::info;

mod config;
mod cronframe;
mod cronjob;
mod job_builder;
mod logger;
//mod tests;
mod utils;
mod web_server;

pub use cronframe::CronFrame;
pub use job_builder::JobBuilder;

// necessary to gather all the annotated jobs automatically
inventory::collect!(JobBuilder<'static>);

#[derive(Debug, Clone)]
pub enum CronJobType {
    Global(fn()),
    Method(fn(arg: Arc<Box<dyn Any + Send + Sync>>)),
    Function(fn()),
}

#[derive(PartialEq, Clone, Copy)]
pub enum CronFilter {
    Global,
    Function,
    Method,
}

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
