use std::sync::Once;

use crate::logger;

static LOGGER_INIT: Once = Once::new();

static mut LOGGER: Option<log4rs::Handle> = None;

pub fn init_logger(path: &str) {
    LOGGER_INIT.call_once(|| {
        unsafe { LOGGER = Some(logger::appender_logger("log/latest.log")) };
        std::fs::remove_file("log/latest.log");
    });

    unsafe{
        if let Some(handle) = &LOGGER{
            
            handle.set_config(logger::appender_config(path))
        }
    }
}