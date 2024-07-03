use std::sync::Once;

use chrono::Duration;

use crate::logger;

static LOGGER_INIT: Once = Once::new();

pub fn init_logger() {
    std::fs::write("log/latest.log", "\n");
    std::thread::sleep(Duration::milliseconds(1000).to_std().unwrap());
    LOGGER_INIT.call_once(|| {
        let _ = logger::appender_logger();
    });
}