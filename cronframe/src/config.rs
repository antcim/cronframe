use rocket::serde::Deserialize;
use std::fs;
use toml;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ConfigData {
    pub webserver: ServerConfig,
    pub logger: LoggerConfig,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerConfig {
    pub port: Option<u16>,
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

pub fn read_config() -> Option<ConfigData>{
    let filename = "cronframe.toml";

    if let Ok(file_content) = fs::read_to_string(filename){
        if let Ok(data) = toml::from_str(&file_content) {
            data
        }else{
            error!("cronframe.toml - data read error");
            None
        }
    }else{
        println!("cronframe.toml - file read error");
        None
    }
}
