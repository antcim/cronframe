use std::sync::Arc;
use rocket_dyn_templates::{context, Template};

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

#[get("/")]
fn home(cronframe: &rocket::State<Arc<CronFrame>>) -> Template {
    let mut available_jobs = vec![];

    for job in cronframe.cron_jobs.lock().unwrap().iter() {
        available_jobs.push(job.name.clone());
    }

    Template::render(
        "index",
        context! {
            cron_jobs: available_jobs,
        },
    )
}

#[get("/job/<name>")]
fn job_info(name: String, cronframe: &rocket::State<Arc<CronFrame>>) -> Template {
    let mut job_info = "not found".to_string();

    for job in cronframe.cron_jobs.lock().unwrap().iter() {
        if job.name == name{
            job_info = job.schedule.to_string()
        }
    }

    Template::render(
        "job",
        context! {
            job_name: name,
            job_info: job_info, 
        },
    )
}