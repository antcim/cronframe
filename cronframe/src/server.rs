use rocket::serde::Serialize;
use rocket_dyn_templates::{context, Template};
use std::sync::Arc;

use crate::CronFrame;

pub fn server(frame: Arc<CronFrame>) -> anyhow::Result<i32> {
    let tokio_runtime = rocket::tokio::runtime::Runtime::new()?;

    let config = rocket::Config {
        port: 8002,
        address: std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
        temp_dir: "templates".into(),
        ..rocket::Config::debug_default()
    };

    let rocket = rocket::custom(&config)
        .mount("/", routes![home, job_info])
        .attach(Template::fairing())
        .manage(frame);

    tokio_runtime.block_on(async move {
        let _ = rocket.launch().await;
    });

    Ok(0)
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
        cron_jobs.push(JobList {
            name: job.name.clone(),
            id: job.id.clone(),
        });
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
    next_schedule: String,
    cron_expression: String,
}

#[get("/job/<name>/<id>")]
fn job_info(name: &str, id: &str, cronframe: &rocket::State<Arc<CronFrame>>) -> Template {
    let mut job_info = JobInfo::default();

    for job in cronframe.cron_jobs.lock().unwrap().iter() {
        if job.name == name && job.id == id {
            job_info = JobInfo {
                name: job.name.clone(),
                id: job.id.clone(),
                r#type: "tbd".to_string(),
                run_id: job.id.clone(),
                status: "tbd".to_string(),
                timeout: job.timeout.unwrap().to_string(),
                schedule: "tbd".to_string(),
                next_schedule: "tbd".to_string(),
                cron_expression: job.schedule.to_string(),
            };
        }
    }

    Template::render("job", context! {job_info})
}
