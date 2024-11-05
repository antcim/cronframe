use chrono::{DateTime, Local, Utc};
use std::{fs::File, io::Write, path::Path};

pub fn utc_to_local_time(utc_time: DateTime<Utc>) -> DateTime<Local> {
    let local_time: DateTime<Local> = DateTime::from(utc_time);
    local_time
}

pub fn home_dir() -> String {
    let tmp = home::home_dir().unwrap();
    tmp.to_str().unwrap().to_owned()
}

pub fn ip_and_port() -> (String, u16) {
    let data = crate::config::read_config().webserver;
    (data.ip, data.port)
}

pub fn gen_template_dir() -> std::io::Result<()> {
    let templ_dir = if std::env::var("CRONFRAME_CLI").is_ok() {
        format!("{}/.cronframe/templates", home_dir())
    } else {
        format!("./templates")
    };

    if !Path::new(&templ_dir).exists() {
        std::fs::create_dir(&templ_dir)?;

        let files = vec![
            ("base.html.tera", crate::web_server::BASE_TEMPLATE),
            ("index.html.tera", crate::web_server::INDEX_TEMPLATE),
            ("job.html.tera", crate::web_server::JOB_TEMPLATE),
            ("tingle.js", crate::web_server::TINGLE_JS),
            ("cronframe.js", crate::web_server::CRONFRAME_JS),
            ("tingle.css", crate::web_server::TINGLE_STYLES),
            ("styles.css", crate::web_server::STYLES),
        ];

        for (file_name, content) in files {
            let mut file = File::create(&format!("{templ_dir}/{file_name}"))?;
            file.write(content.as_bytes())?;
            file.sync_all()?;
        }
    }
    Ok(())
}
