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
    let data = read_config().webserver;
    (data.ip, data.port)
}
