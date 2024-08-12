//! Utilities

use chrono::{DateTime, Local, Utc};

/// Convertion from UTC time to local time
pub fn local_time(utc_time: DateTime<Utc>) -> DateTime<Local>{
    let local_time: DateTime<Local> = DateTime::from(utc_time);
    local_time
}

pub fn home_dir() -> String{
    let tmp = home::home_dir().unwrap();
    tmp.to_str().unwrap().to_owned()
}