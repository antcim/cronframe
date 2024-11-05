use std::{fs, path::Path, time::Duration};

use crate::{
    config::read_config,
    web_server::{
        BASE_TEMPLATE, CRONFRAME_JS, INDEX_TEMPLATE, JOB_TEMPLATE, STYLES, TINGLE_JS, TINGLE_STYLES,
    },
};
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

/// - base.html.tera
/// - index.htm.tera
/// - job.html.tera
/// - tingle.js
/// - cronframe.js
/// - styles.css
/// - tingle.css
pub fn generate_template_dir() {
    if std::env::var("CRONFRAME_CLI").is_ok() {
        let home_dir = home_dir();

        if !Path::new(&format!("{home_dir}/.cronframe/templates")).exists() {
            fs::create_dir(format!("{home_dir}/.cronframe/templates"))
                .expect("could not create templates directory");

            let _ = fs::write(
                Path::new(&format!("{home_dir}/.cronframe/templates/base.html.tera")),
                BASE_TEMPLATE,
            );
            let _ = fs::write(
                Path::new(&format!("{home_dir}/.cronframe/templates/index.html.tera")),
                INDEX_TEMPLATE,
            );
            let _ = fs::write(
                Path::new(&format!("{home_dir}/.cronframe/templates/job.html.tera")),
                JOB_TEMPLATE,
            );
            let _ = fs::write(
                Path::new(&format!("{home_dir}/.cronframe/templates/tingle.js")),
                TINGLE_JS,
            );
            let _ = fs::write(
                Path::new(&format!("{home_dir}/.cronframe/templates/cronframe.js")),
                CRONFRAME_JS,
            );
            let _ = fs::write(
                Path::new(&format!("{home_dir}/.cronframe/templates/tingle.css")),
                TINGLE_STYLES,
            );
            let _ = fs::write(
                Path::new(&format!("{home_dir}/.cronframe/templates/styles.css")),
                STYLES,
            );
        }
    } else {
        if !Path::new(&format!("./templates")).exists() {
            fs::create_dir(format!("templates")).expect("could not create templates directory");

            let _ = fs::write(
                Path::new(&format!("./templates/base.html.tera")),
                BASE_TEMPLATE,
            );
            let _ = fs::write(
                Path::new(&format!("./templates/index.html.tera")),
                INDEX_TEMPLATE,
            );
            let _ = fs::write(
                Path::new(&format!("./templates/job.html.tera")),
                JOB_TEMPLATE,
            );
            let _ = fs::write(Path::new(&format!("./templates/tingle.js")), TINGLE_JS);
            let _ = fs::write(
                Path::new(&format!("./templates/cronframe.js")),
                CRONFRAME_JS,
            );
            let _ = fs::write(Path::new(&format!("./templates/tingle.css")), TINGLE_STYLES);
            let _ = fs::write(Path::new(&format!("./templates/styles.css")), STYLES);
        }
    }
    std::thread::sleep(Duration::from_secs(10));
}
