use std::sync::Once;

use crate::logger;

static LOGGER_INIT: Once = Once::new();

pub fn init_logger() {
    LOGGER_INIT.call_once(|| {
        let _ = logger::rolling_logger();
    });
}
