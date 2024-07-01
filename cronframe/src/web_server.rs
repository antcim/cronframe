use crate::{
    config::read_config,
    cronframe::{self, CronFilter, CronFrame},
    CronJobType,
};
use log::info;
use rocket::{config::Shutdown, futures::FutureExt, serde::Serialize};
use rocket_dyn_templates::{context, Template};
use std::sync::Arc;

pub fn web_server(frame: Arc<CronFrame>) {
    let cronframe = frame.clone();

    let tokio_runtime = rocket::tokio::runtime::Runtime::new().unwrap();

    let config = match read_config() {
        Some(config_data) => rocket::Config {
            port: config_data.server.port,
            address: std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
            temp_dir: "templates".into(),
            shutdown: Shutdown {
                ctrlc: false,
                ..Default::default()
            },
            cli_colors: false,
            ..rocket::Config::release_default()
        },
        None => {
            // default config
            rocket::Config {
                port: 8002,
                address: std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
                temp_dir: "templates".into(),
                shutdown: Shutdown {
                    ctrlc: false,
                    ..Default::default()
                },
                cli_colors: false,
                ..rocket::Config::release_default()
            }
        }
    };

    let rocket = rocket::custom(&config)
        .mount(
            "/",
            routes![styles, home, job_info, update_timeout, update_schedule],
        )
        .attach(Template::fairing())
        .manage(frame);

    let (tx, rx) = cronframe.web_server_channels.clone();

    println!("HERE 0");

    tokio_runtime.block_on(async move {
        let rocket = rocket.ignite().await;
        let shutdown_handle = rocket.as_ref().unwrap().shutdown();
        println!("SENDING STUFF NOW!!!");
        let _ = tx.send(shutdown_handle);
        println!(
            "CronFrame running at http://{}:{}",
            config.address, config.port
        );
        let _ = rocket.unwrap().launch().await;
    });
}

#[get("/styles")]
async fn styles() -> Result<rocket::fs::NamedFile, std::io::Error> {
    rocket::fs::NamedFile::open("templates/styles.css").await
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct JobList {
    name: String,
    id: String,
}

#[get("/")]
fn home(cronframe: &rocket::State<Arc<CronFrame>>) -> Template {
    let mut cron_jobs = vec![];

    for job in cronframe.cron_jobs.lock().unwrap().iter() {
        let job_type = match job.job {
            CronJobType::Global(_) => CronFilter::Global,
            CronJobType::Function(_) => CronFilter::Function,
            CronJobType::Method(_) => CronFilter::Method,
        };

        if cronframe.filter.is_none() || cronframe.filter == Some(job_type) {
            cron_jobs.push(JobList {
                name: job.name.clone(),
                id: job.id.to_string(),
            });
        }
    }

    Template::render("index", context! {cron_jobs})
}

#[derive(Serialize, Default)]
#[serde(crate = "rocket::serde")]
struct JobInfo {
    name: String,
    id: String,
    r#type: String,
    run_id: String,
    status: String,
    timeout: String,
    schedule: String,
    upcoming: String,
    fail: bool,
}

#[get("/job/<name>/<id>")]
fn job_info(name: &str, id: &str, cronframe: &rocket::State<Arc<CronFrame>>) -> Template {
    let mut job_info = JobInfo::default();

    for job in cronframe.cron_jobs.lock().unwrap().iter() {
        if job.name == name && job.id.to_string() == id {
            job_info = JobInfo {
                name: job.name.clone(),
                id: job.id.to_string(),
                r#type: match job.job {
                    CronJobType::Global(_) => "Global".to_string(),
                    CronJobType::Function(_) => "Function".to_string(),
                    CronJobType::Method(_) => "Method".to_string(),
                },
                run_id: job.get_run_id(),
                status: job.status(),
                timeout: if job.timeout.is_some() {
                    job.timeout.unwrap().to_string()
                } else {
                    "None".into()
                },
                schedule: job.schedule(),
                upcoming: job.upcoming(),
                fail: job.failed,
            };
            break;
        }
    }

    Template::render("job", context! {job_info})
}

#[get("/job/<name>/<id>/toutset/<value>")]
fn update_timeout(name: &str, id: &str, value: i64, cronframe: &rocket::State<Arc<CronFrame>>) {
    for job in cronframe.cron_jobs.lock().unwrap().iter_mut() {
        if job.name == name && job.id.to_string() == id {
            let job_id = format!("{} ID#{}", job.name, job.id);
            job.start_time = None;
            job.set_timeout(value);
            info!("job @{job_id} - Timeout Update");
        }
    }
}

#[get("/job/<name>/<id>/schedset/<expression>")]
fn update_schedule(
    name: &str,
    id: &str,
    expression: &str,
    cronframe: &rocket::State<Arc<CronFrame>>,
) {
    for job in cronframe.cron_jobs.lock().unwrap().iter_mut() {
        if job.name == name && job.id.to_string() == id {
            let job_id = format!("{} ID#{}", job.name, job.id);
            if job.set_schedule(expression) {
                info!("job @{job_id} - Schedule Update");
            } else {
                info!("job @{job_id} - Schedule Update Fail - Cron Expression Parse Error");
            }
        }
    }
}
