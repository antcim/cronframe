use rocket::serde::Deserialize;
use std::fs;
use toml;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ConfigData {
    pub server: ServerConfig,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerConfig {
    pub port: u16,
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
