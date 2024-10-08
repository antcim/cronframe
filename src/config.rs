//! Configuration avaliable in `cronframe.toml`

use crate::utils;
use rocket::serde::Deserialize;
use std::fs;
use toml;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ConfigData {
    pub webserver: Option<ServerConfig>,
    pub logger: Option<LoggerConfig>,
    pub scheduler: Option<SchedulerConfig>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerConfig {
    pub port: Option<u16>,
    pub ip: Option<String>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoggerConfig {
    pub dir: Option<String>,
    pub file_size: Option<u64>,
    pub archive_files: Option<u32>,
    pub latest_file_name: Option<String>,
    pub archive_file_name: Option<String>,
    pub msg_pattern: Option<String>,
    pub level_filter: Option<String>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SchedulerConfig {
    pub grace: Option<u32>,
}

/// This function reads cronframe configuration data from the `cronframe.toml` file.
///
/// There are three sections to the configuration:
/// - webserver
/// - logger
/// - scheduler
///
/// ```toml
/// [webserver]
/// port = 8098
///
/// [logger]
/// dir = "log"
/// file_size = 1 # this is in MB
/// archive_files = 3
/// latest_file_name = "latest"
/// archive_file_name = "archive"
/// msg_pattern = "{l} {t} - {m}{n}"
/// level_filter = "info"
///
/// [scheduler]
/// grace = 250 # this is in ms
/// ```
///
pub fn read_config() -> Option<ConfigData> {
    let filename = if std::env::var("CRONFRAME_CLI").is_ok() {
        let home_dir = utils::home_dir();
        &format!("{home_dir}/.cronframe/cronframe.toml")
    } else {
        "cronframe.toml"
    };

    if let Ok(file_content) = fs::read_to_string(filename) {
        if let Ok(data) = toml::from_str(&file_content) {
            data
        } else {
            error!("cronframe.toml - data read error");
            None
        }
    } else {
        info!("cronframe.toml - file not found");
        None
    }
}
