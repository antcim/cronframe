use crate::{utils, CronFilter};
use rocket::serde::Deserialize;
use std::fs;
use toml;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ConfigData {
    pub webserver: ServerConfig,
    pub logger: LoggerConfig,
    pub scheduler: SchedulerConfig,
}

impl Default for ConfigData {
    fn default() -> Self {
        ConfigData {
            webserver: ServerConfig::default(),
            logger: LoggerConfig::default(),
            scheduler: SchedulerConfig::default(),
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ConfigDataToml {
    pub webserver: Option<ServerConfigToml>,
    pub logger: Option<LoggerConfigToml>,
    pub scheduler: Option<SchedulerConfigToml>,
}

impl ConfigDataToml {
    fn to_config_data(self) -> ConfigData {
        ConfigData {
            webserver: {
                if self.webserver.is_some() {
                    ServerConfig {
                        port: self
                            .webserver
                            .as_ref()
                            .unwrap()
                            .port
                            .unwrap_or_else(|| 8098),
                        ip: self
                            .webserver
                            .unwrap()
                            .ip
                            .unwrap_or_else(|| "127.0.0.1".to_string()),
                    }
                } else {
                    ServerConfig::default()
                }
            },
            logger: {
                if self.logger.is_some() {
                    let data = self.logger.unwrap();
                    LoggerConfig {
                        enabled: data.enabled.unwrap_or_else(|| true),
                        dir: data.dir.unwrap_or_else(|| "log".to_string()),
                        file_size: data.file_size.unwrap_or_else(|| 1),
                        archive_files: data.archive_files.unwrap_or_else(|| 3),
                        latest_file_name: data
                            .latest_file_name
                            .unwrap_or_else(|| "latest".to_string()),
                        archive_file_name: data
                            .archive_file_name
                            .unwrap_or_else(|| "archive_".to_string()),
                        msg_pattern: data.msg_pattern.unwrap_or_else(|| {
                            "{d(%Y-%m-%d %H:%M:%S %Z)} {l} {t} - {m}{n}".to_string()
                        }),
                        level_filter: data.level_filter.unwrap_or_else(|| "info".to_string()),
                    }
                } else {
                    LoggerConfig::default()
                }
            },
            scheduler: {
                if self.scheduler.is_some() {
                    let data = self.scheduler.unwrap();
                    SchedulerConfig {
                        job_filter: data.job_filter.unwrap_or_else(|| CronFilter::None),
                        grace: data.grace.unwrap_or_else(|| 250),
                    }
                } else {
                    SchedulerConfig::default()
                }
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerConfig {
    pub port: u16,
    pub ip: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: 8098,
            ip: "127.0.0.1".to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerConfigToml {
    pub port: Option<u16>,
    pub ip: Option<String>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoggerConfig {
    pub enabled: bool,
    pub dir: String,
    pub file_size: u64,
    pub archive_files: u32,
    pub latest_file_name: String,
    pub archive_file_name: String,
    pub msg_pattern: String,
    pub level_filter: String,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        LoggerConfig {
            enabled: true,
            dir: "log".to_string(),
            file_size: 1,
            archive_files: 3,
            latest_file_name: "latest".to_string(),
            archive_file_name: "archive".to_string(),
            msg_pattern: "{d(%Y-%m-%d %H:%M:%S %Z)} {l} {t} - {m}{n}".to_string(),
            level_filter: "info".to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoggerConfigToml {
    pub enabled: Option<bool>,
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
    pub job_filter: CronFilter,
    pub grace: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        SchedulerConfig {
            job_filter: CronFilter::None,
            grace: 250,
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SchedulerConfigToml {
    pub job_filter: Option<CronFilter>,
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
pub fn read_config() -> ConfigData {
    let filename = if std::env::var("CRONFRAME_CLI").is_ok() {
        let home_dir = utils::home_dir();
        &format!("{home_dir}/.cronframe/cronframe.toml")
    } else {
        "cronframe.toml"
    };

    if let Ok(file_content) = fs::read_to_string(filename) {
        if let Ok(data) = toml::from_str::<ConfigDataToml>(&file_content) {
            data.to_config_data()
        } else {
            error!("cronframe.toml - data read error");
            ConfigData::default()
        }
    } else {
        info!("cronframe.toml - file not found");
        ConfigData::default()
    }
}
