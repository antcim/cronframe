//! Utilities

use crate::config::read_config;
use chrono::{DateTime, Local, Utc};

/// Convertion from UTC to Local time
pub fn local_time(utc_time: DateTime<Utc>) -> DateTime<Local> {
    let local_time: DateTime<Local> = DateTime::from(utc_time);
    local_time
}

pub fn home_dir() -> String {
    let tmp = home::home_dir().unwrap();
    tmp.to_str().unwrap().to_owned()
}

pub fn ip_and_port() -> (String, u16) {
    match read_config() {
        Some(config_data) => {
            if let Some(webserver_data) = config_data.webserver {
                (
                    webserver_data.ip.unwrap_or_else(|| "127.0.0.1".to_string()),
                    webserver_data.port.unwrap_or_else(|| 8098),
                )
            } else {
                ("localhost".to_string(), 8098)
            }
        }
        None => ("localhost".to_string(), 8098),
    }
}
