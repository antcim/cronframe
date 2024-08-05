//! Custom setup of rocket.rs for the Cronframe web server

use crate::{
    config::read_config,
    cronframe::CronFrame,
    CronFilter, CronJobType,
};
use log::info;
use rocket::{
    config::Shutdown, serde::Serialize,
};
use rocket_dyn_templates::{context, Template};
use std::{fs, sync::Arc, time::Duration};

/// Called by the init funciton of the Cronframe type for setting up the web server
///
/// It provides 7 routes, five of which are API only.
///
/// Upon first start of the library it will generate a templates folder inside the current director with the following files:
/// - base.html.tera
/// - index.htm.tera
/// - job.html.tera
/// - tingle.js
/// - cronframe.js
/// - styles.css
/// - tingle.css
pub fn web_server(frame: Arc<CronFrame>) {
    if !std::path::Path::new("./templates").exists() {
        println!("Generating templates directory content...");
        fs::create_dir("templates").expect("could not create templates directory");
        let _ = fs::write(
            std::path::Path::new("./templates/base.html.tera"),
            BASE_TEMPLATE,
        );
        let _ = fs::write(
            std::path::Path::new("./templates/index.html.tera"),
            INDEX_TEMPLATE,
        );
        let _ = fs::write(
            std::path::Path::new("./templates/job.html.tera"),
            JOB_TEMPLATE,
        );
        let _ = fs::write(
            std::path::Path::new("./templates/tingle.js"),
            TINGLE_JS,
        );
        let _ = fs::write(
            std::path::Path::new("./templates/cronframe.js"),
            CRONFRAME_JS,
        );
        let _ = fs::write(std::path::Path::new("./templates/tingle.css"), TINGLE_STYLES);
        let _ = fs::write(std::path::Path::new("./templates/styles.css"), STYLES);
        std::thread::sleep(Duration::from_secs(10));
    }

    let cronframe = frame.clone();

    let tokio_runtime = rocket::tokio::runtime::Runtime::new().unwrap();

    let config = match read_config() {
        Some(config_data) => rocket::Config {
            port: {
                if let Some(webserver_data) = &config_data.webserver {
                    webserver_data.port.unwrap_or_else(|| 8098)
                } else {
                    8098
                }
            },
            address: {
                if let Some(webserver_data) = config_data.webserver {
                    webserver_data.ip.unwrap_or_else(|| "127.0.0.1".to_string())
                } else {
                    "127.0.0.1".to_string()
                }
                .parse()
                .unwrap()
            },
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
                port: 8098,
                address: std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
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
            routes![
                styles,
                cronframe,
                tingle,
                tinglejs,
                home,
                job_info,
                update_timeout,
                update_schedule,
                suspension_handle,
                start_scheduler,
                stop_scheduler,
            ],
        )
        .attach(Template::fairing())
        .manage(frame);

    let (tx, _) = cronframe.web_server_channels.clone();

    tokio_runtime.block_on(async move {
        let rocket = rocket.ignite().await;

        let shutdown_handle = rocket
            .as_ref()
            .expect("rocket unwrap error in web server init")
            .shutdown();

        let _ = tx.send(shutdown_handle);

        println!(
            "CronFrame running at http://{}:{}",
            config.address, config.port
        );

        let _ = rocket
            .expect("rocket unwrap error in web server launch")
            .launch()
            .await;
    });
}

// necessary to have somewhat decent-looking pages
#[get("/styles")]
async fn styles() -> Result<rocket::fs::NamedFile, std::io::Error> {
    rocket::fs::NamedFile::open("templates/styles.css").await
}

// necessary to have somewhat decent-looking pages
#[get("/tingle")]
async fn tingle() -> Result<rocket::fs::NamedFile, std::io::Error> {
    rocket::fs::NamedFile::open("templates/tingle.css").await
}

// necessary to have somewhat functioning pages
#[get("/tinglejs")]
async fn tinglejs() -> Result<rocket::fs::NamedFile, std::io::Error> {
    rocket::fs::NamedFile::open("templates/tingle.js").await
}

// necessary to have somewhat functioning pages
#[get("/cronframejs")]
async fn cronframe() -> Result<rocket::fs::NamedFile, std::io::Error> {
    rocket::fs::NamedFile::open("templates/cronframe.js").await
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct JobList {
    name: String,
    id: String,
}

// homepage returning a list of al jobs in the following categories: active, timed-out, suspended
#[get("/")]
fn home(cronframe: &rocket::State<Arc<CronFrame>>) -> Template {
    let running = *cronframe.running.lock().unwrap();

    let mut active_jobs = vec![];
    let mut timedout_jobs = vec![];
    let mut suspended_jobs = vec![];

    for job in cronframe
        .cron_jobs
        .lock()
        .expect("cron jobs unrwap error in web server")
        .iter()
    {
        let job_type = match job.job {
            CronJobType::Global(_) => CronFilter::Global,
            CronJobType::Function(_) => CronFilter::Function,
            CronJobType::Method(_) => CronFilter::Method,
        };

        if cronframe.filter.is_none() || cronframe.filter == Some(job_type) {
            if job.status() == "Suspended" {
                suspended_jobs.push(JobList {
                    name: job.name.clone(),
                    id: job.id.to_string(),
                });
            } else if job.status() == "Timed-Out" {
                timedout_jobs.push(JobList {
                    name: job.name.clone(),
                    id: job.id.to_string(),
                });
            } else {
                active_jobs.push(JobList {
                    name: job.name.clone(),
                    id: job.id.to_string(),
                });
            }
        }
    }

    Template::render("index", context! {running, active_jobs, timedout_jobs, suspended_jobs})
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
    upcoming_utc: String,
    upcoming_local: String,
    fail: bool,
}

// job page information where it is possilbe to change, schedule, timeout and toggle scheduling suspension
#[get("/job/<name>/<id>")]
fn job_info(name: &str, id: &str, cronframe: &rocket::State<Arc<CronFrame>>) -> Template {
    let running = *cronframe.running.lock().unwrap();
    let mut job_info = JobInfo::default();

    for job in cronframe.cron_jobs.lock().unwrap().iter() {
        if job.name == name && job.id.to_string() == id {
            job_info = JobInfo {
                name: job.name.clone(),
                id: job.id.to_string(),
                r#type: job.type_to_string(),
                run_id: job.get_run_id(),
                status: job.status(),
                timeout: job.timeout_to_string(),
                schedule: job.schedule(),
                upcoming_utc: job.upcoming_utc(),
                upcoming_local: job.upcoming_local(),
                fail: job.failed,
            };
            break;
        }
    }

    Template::render("job", context! {running, job_info})
}

// API route to change the value of the timeout
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

// API route to change the value of the cron expression and therefore the schedule
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

// API route to toggle the scheduling suspension for a job
#[get("/job/<name>/<id>/suspension_toggle")]
fn suspension_handle(name: &str, id: &str, cronframe: &rocket::State<Arc<CronFrame>>) {
    for job in cronframe.cron_jobs.lock().unwrap().iter_mut() {
        if job.name == name && job.id.to_string() == id {
            let job_id = format!("{} ID#{}", job.name, job.id);
            if !job.suspended {
                job.suspended = true;
                info!("job @{job_id} - Scheduling Suspended");
            } else {
                job.suspended = false;
                info!("job @{job_id} - Scheduling Reprised");
            }
        }
    }
}

// API route to start the scheduler
#[get("/start_scheduler")]
fn start_scheduler(cronframe: &rocket::State<Arc<CronFrame>>) {
    cronframe.start_scheduler();
}

// API route to stop the scheduler
#[get("/stop_scheduler")]
fn stop_scheduler(cronframe: &rocket::State<Arc<CronFrame>>) {
    cronframe.stop_scheduler();
}

// templates folder data: templates/base.tera.html
const BASE_TEMPLATE: &str = {
  r#"<!DOCTYPE html>
<html class="light-mode">

<head>
    <meta charset="utf-8" />
    <title>CronFrame</title>
    <link href='https://fonts.googleapis.com/css?family=Lato' rel='stylesheet'>
    <link rel="stylesheet" href="/styles">
    <link rel="stylesheet" href="/tingle">

</head>

<body>

    <div id="wrapper">
        <div>
            <div id="barContainer">
                <div id="progressBar">
                    <div id="barStatus"></div>
                </div>
            </div>
            <div id="container">
                <header>
                    <div id="logo">
                        <a href="/"><span class="cron">Cron</span><span style="color:#FF3D00">Frame</span></a>
                    </div>

                    <label class="switch" title="Toggle Color Theme">
                        <input type="checkbox" onchange="toggleMode()" id="slider">
                        <span class="slider"></span>
                    </label>
                    {% if running %}
                    <div id="scheduler_status_running">
                        <span style="padding:0px 5px">ⓘ</span> Scheduler Running
                    </div>
                    <div id="scheduler_stop" onclick="stopScheduler()">
                        <svg height="24" viewBox="0 0 24 20" width="24" xmlns="http://www.w3.org/2000/svg">
                            <path fill="currentColor"
                                d="M12 21c4.411 0 8-3.589 8-8 0-3.35-2.072-6.221-5-7.411v2.223A6 6 0 0 1 18 13c0 3.309-2.691 6-6 6s-6-2.691-6-6a5.999 5.999 0 0 1 3-5.188V5.589C6.072 6.779 4 9.65 4 13c0 4.411 3.589 8 8 8z" />
                            <path fill="currentColor" d="M11 2h2v10h-2z" />
                        </svg>
                    </div>
                    {% else %}
                    <div id="scheduler_status_not_running">
                        <span style="padding:0px 5px">ⓘ</span> Scheduler Not Running
                    </div>
                    <div id="scheduler_start" onclick="startScheduler()">
                        <svg height="24" viewBox="0 0 24 20" width="24" xmlns="http://www.w3.org/2000/svg">
                            <path fill="currentColor"
                                d="M12 21c4.411 0 8-3.589 8-8 0-3.35-2.072-6.221-5-7.411v2.223A6 6 0 0 1 18 13c0 3.309-2.691 6-6 6s-6-2.691-6-6a5.999 5.999 0 0 1 3-5.188V5.589C6.072 6.779 4 9.65 4 13c0 4.411 3.589 8 8 8z" />
                            <path fill="currentColor" d="M11 2h2v10h-2z" />
                        </svg>
                    </div>
                    {% endif %}
                </header>

                <div id="content">
                    {% block content %}
                    {% endblock content %}
                </div>

                <footer>
                    <label class="reload">
                        <input type="checkbox" onchange="toggleReload()" id="autoreload">
                        <span class="check"></span>
                    </label>
                    5s Reload
                    <a href="https://github.com/antcim/cronframe" target="_blank" class="repo" title="Developed by Antonio Cimino">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
                            <path fill="currentColor"
                                d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                        </svg>
                        <span>antcim/cronframe</span>
                    </a>
                </footer>
                <script src="/tinglejs"></script>
                <script src="/cronframejs"></script>
            </div>
        </div>
    </div>
</body>

</html>"#
};

// templates folder data: templates/index.tera.html
const INDEX_TEMPLATE: &str = {
    r#"{% extends "base" %}

{% block content %}
<table id="job_list">
    <tr>
        <th>
            Active Jobs
            <div class="refresh" onclick="reloadPage()">⟳</div>
        </th>
    </tr>
    {% if active_jobs %}
    {% for cron_job in active_jobs %}
    {% set activelink = "/job/" ~ cron_job.name ~ "/" ~ cron_job.id %}
    <tr>
        <td><a href="{{activelink}}">{{cron_job.name}}</a></td>
        <td>{{cron_job.id}}</td>
    </tr>
    {% endfor %}
    {% else %}
    <tr>
        <td>No active job found</td>
    </tr>
    {% endif %}

</table>

<table id="job_list">
    <tr>
        <th>
            Timed-Out Jobs <div class="refresh" onclick="reloadPage()">⟳</div>
        </th>
    </tr>
    {% if timedout_jobs %}
    {% for cron_job in timedout_jobs %}
    {% set timedoutlink = "/job/" ~ cron_job.name ~ "/" ~ cron_job.id %}
    <tr>
        <td><a href="{{timedoutlink}}">{{cron_job.name}}</a></td>
        <td>{{cron_job.id}}</td>
    </tr>
    {% endfor %}
    {% else %}
    <tr>
        <td>No timed-out job found</td>
    </tr>
    {% endif %}
</table>

<table id="job_list">
    <tr>
        <th>
            Suspended Jobs <div class="refresh" onclick="reloadPage()">⟳</div>
        </th>
    </tr>
    {% if suspended_jobs %}
    {% for cron_job in suspended_jobs %}
    {% set suspendedlink = "/job/" ~ cron_job.name ~ "/" ~ cron_job.id %}
    <tr>
        <td><a href="{{suspendedlink}}">{{cron_job.name}}</a></td>
        <td>{{cron_job.id}}</td>
    </tr>
    {% endfor %}
    {% else %}
    <tr>
        <td>No suspended job found</td>
    </tr>
    {% endif %}
</table>
{% endblock content %}"#
};

// templates folder data: templates/job.tera.html
const JOB_TEMPLATE: &str = {
    r#"{% extends "base" %}

{% block content %}

{% if job_info.name != ""%}
<table id="job_info">
    <tr>
        <th colspan="2">
            Job Info @{{job_info.name}} <div class="refresh" onclick="reloadPage()">⟳</div>
        </th>
    </tr>
    <tr>
        <td>Name</td>
        <td colspan="2">{{job_info.name}}</td>
    </tr>
    <tr>
        <td>Id</td>
        <td colspan="2">
            <div class="id_cont">
                <span id="job_id">{{job_info.id}}</span>
                <span class="clipboard" onclick="copyToClipBoard('job_id')" title="copy to clipboard">
                    <?xml version="1.0" ?>
                    <svg width="16px" height="16px" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
                        <path
                            d="M464 0c26.51 0 48 21.49 48 48v288c0 26.51-21.49 48-48 48H176c-26.51 0-48-21.49-48-48V48c0-26.51 21.49-48 48-48h288M176 416c-44.112 0-80-35.888-80-80V128H48c-26.51 0-48 21.49-48 48v288c0 26.51 21.49 48 48 48h288c26.51 0 48-21.49 48-48v-48H176z" />
                    </svg>
                </span>
            </div>
        </td>
    </tr>
    <tr>
        <td>Type</td>
        <td colspan="2">{{job_info.type}} Job</td>
    </tr>
    {% if job_info.run_id != "None" %}
    <tr>
        <td>Run Id</td>
        <td colspan="2">
            <div class="id_cont">
                <span id="run_id">{{job_info.run_id}}</span>
                <span class="clipboard" onclick="copyToClipBoard('run_id')" title="copy to clipboard">
                    <?xml version="1.0" ?>
                    <svg width="16px" height="16px" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
                        <path
                            d="M464 0c26.51 0 48 21.49 48 48v288c0 26.51-21.49 48-48 48H176c-26.51 0-48-21.49-48-48V48c0-26.51 21.49-48 48-48h288M176 416c-44.112 0-80-35.888-80-80V128H48c-26.51 0-48 21.49-48 48v288c0 26.51 21.49 48 48 48h288c26.51 0 48-21.49 48-48v-48H176z" />
                    </svg>
                </span>
            </div>
        </td>
    </tr>
    {% endif %}
    <tr>
        <td>Status</td>
        <td colspan="">
            {% if job_info.status == "Timed-Out" or job_info.status == "Suspended" %}
            <div class="line_status_gray">{{job_info.status}}</div>
            {% elif job_info.status == "Running" %}
            <div class="line_status_green">{{job_info.status}}</div>
            {% else %}
            <div class="line_status_yellow">{{job_info.status}}</div>
            {% endif %}
        </td>
        <td>
            {% if job_info.status != "Suspended" %}
            <button onclick="suspensionHandle()">Suspend Scheduling</button>
            {% else %}
            <button onclick="suspensionHandle()">Reprise Scheduling</button>
            {% endif %}
        </td>
    </tr>
    <tr>
        <td>Fail History</td>
        <td colspan="2">
            {% if job_info.fail %}
            <div class="line_status_orange">Failed instances recorded</div>
            {% else %}
            No failed instances recorded
            {% endif %}
        </td>
    </tr>
    <tr>
        <td>Schedule</td>
        <td>
            {{job_info.schedule}}
        </td>
        <td>
            <input oninput="setSchedule(this.value)" type="text" placeholder="enter cron expression">
            <button onclick="updateSchedule()">Update</button>
        </td>
    </tr>
    <tr>
        <td>Timeout</td>
        <td>
            {{job_info.timeout}}
        </td>
        <td>
            <input oninput="setTimeout(this.value)" type="number" min="0" placeholder="enter timout in ms">
            <button onclick="updateTimeout()">Update</button>
        </td>
    </tr>
    <tr>
        <td>Upcoming</td>
        <td colspan="2">
            {% if job_info.upcoming_utc == "None due to timeout." %}
                <p>{{job_info.upcoming_utc}}</p>
            {% else %}
                <p>{{job_info.upcoming_utc}}</p>
                {% if job_info.status != "Timed-Out" and job_info.status != "Suspended" %}
                <p>{{job_info.upcoming_local}} (Local)</p>
                {% endif %}
            {% endif %}
        </td>
    </tr>
</table>

<script>
    
</script>

{% else %}
<div id="job_info">
    <div class="job_info_item">
        Job not found
    </div>
</div>
{% endif %}
{% endblock content %}"#
};

// templates folder data: templates/styles.css
const STYLES: &str = {
  r#":root {
  --dark-orange: #ff3d00;
  --light-orange: #ffa702;
  --dark-green: #0fa702;
  --green: green;
  --red: red;
}

.light-mode {
  --body-bg: #f6f6f6;
  --container-bg: #ffffff;
  --content-bg: #f1f1f1;
  --font-color: #494949;
  --gray-status-color: rgba(0, 0, 0, .3);
  --checkbox: rgba(0, 0, 0, .1);
  --scheduler-status-stop: rgba(0, 0, 0, .1);
  --scheduler-start-stop-text:  rgba(0,0,0,.3);
  --scheduler-status-running: rgba(51, 255, 0, .3);
  --scheduler-status-running-text: rgba(0, 0, 0, .4);
  --scheduler-status-not-running: rgba(237, 233, 157, .7);
  --scheduler-status-not-running-text: rgba(0, 0, 0, .4);
}

.dark-mode {
  --body-bg: #161616;
  --container-bg: #2c2c2c;
  --content-bg: #212121;
  --font-color: #9a9a9a;
  --gray-status-color: rgba(255, 255, 255, .3);
  --checkbox: rgba(0, 0, 0, .3);
  --scheduler-status-stop: rgba(0, 0, 0, .3);
  --scheduler-start-stop-text:  rgba(255,255,255,.3);
  --scheduler-status-running: rgba(51, 255, 0, .2);
  --scheduler-status-running-text: rgba(255, 255, 255, .4);
  --scheduler-status-not-running: rgba(237, 233, 157, .3);
  --scheduler-status-not-running-text: rgba(255, 255, 255, .4);
}

body {
  background: var(--body-bg);
  color: var(--font-color);
  font-family: "Lato"!important;
  margin: 0;
}

a {
  text-decoration: none;
}

a:link {
  color: var(--dark-orange);
}

a:visited {
  color: var(--light-orange);
}

a:hover {
  color: var(--green);
}

a:active {
  color: var(--red);
}

header {
  display: flex;
  padding: 15px;
  align-items: center;
  justify-content: center;
}

footer {
  padding: 15px;
}

#logo {
  flex: 2;
  font-weight: bold;
  font-size: 30pt;
}

.cron {
  color: var(--font-color);
}

.refresh {
  display: inline;
  margin-left: 6px;
}

.refresh:hover {
  cursor: pointer;
  color: var(--light-orange);
}

.refresh:active {
  cursor: pointer;
  color: var(--dark-orange);
}

#wrapper {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  height: 100vh;
  marign: 0;
  padding: 0;
}

#container {
  display: flex;
  flex-direction: column;
  gap: 5px;
  background: var(--container-bg);
  padding: 10px;
  padding-top: 5px;
  padding-bottomn: 5px;
  border--radius: 6px;
  box-shadow: 0px 0px 3px 0px rgba(0, 0, 0, .1);
  max-width: 1200px;
  min-width: 500px;
}

#scheduler_start, #scheduler_stop{
  color: var(--scheduler-start-stop-text);
  margin-left: 10px;
  font-size: 20pt;
  font-weight: bold;
  cursor: pointer;
  background: var(--scheduler-status-stop);
  border-radius: 6px;
  padding: 5px 8px;
}

#scheduler_start:hover{
  color: var(--scheduler-status-running-text);
  background: var(--scheduler-status-running);
}

#scheduler_start:active{
  color: white;
  background: var(--green);
}

#scheduler_stop:hover{
  color: var(--scheduler-status-running-text);
  background: var(--light-orange);
}

#scheduler_stop:active{
  color: white;
  background: var(--dark-orange);
}

#scheduler_status_running {
  font-weight: bold;
  background: var(--scheduler-status-running);
  padding: 10px;
  border-radius: 6px;
  margin-left: 10px;
  color: var(--scheduler-status-running-text);
}

#scheduler_status_not_running {
  font-weight: bold;
  background: var(--scheduler-status-not-running);
  padding: 10px;
  border-radius: 6px;
  margin-left: 10px;
  color: var(--scheduler-status-not-running-text);
}

#content {
  background: var(--content-bg);
  padding: 10px;
  border-radius: 6px;
  max-height: 70vh;
  overflow: auto;
}

input[type=text],
input[type=number] {
  padding: 10px;
  display: inline-block;
  border: 1px solid #ccc;
  border-radius: 4px;
  box-sizing: border-box;
}

input[type=text]:focus,
input[type=number]:focus {
  border-color: rgba(229, 103, 23, 0.7);
  box-shadow: 0 1px 1px rgba(229, 103, 23, 0.075) inset, 0 0 4px rgba(229, 103, 23, 0.6);
  outline: 0 none;
}

button {
  background-color: rgba(0, 0, 0, .5);
  font-weight: bold;
  color: white;
  padding: 12px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  box-shadow: 0 0 2px 1px rgba(0, 0, 0, .1) inset;
}

button:hover {
  background-color: #4caf50;
  color: white;
  border: none;
  cursor: pointer;
  box-shadow: 0 0 2px 1px rgba(0, 0, 0, .1) inset;
}

button:active {
  background-color: var(--dark-orange);
  color: white;
  border: none;
  cursor: pointer;
  box-shadow: 0 0 2px 1px rgba(0, 0, 0, .2) inset;
}

.line_status_green {
  font-weight: bold;
  display: inline-block;
  background: rgba(51, 255, 0, .5);
  padding: 10px;
  border-radius: 6px;
  border: 1px solid rgba(0, 0, 0, .1);
  color: rgba(0, 0, 0, .4);
}

.line_status_yellow {
  font-weight: bold;
  display: inline-block;
  background: rgba(255, 236, 102, 1);
  padding: 10px;
  border-radius: 6px;
  border: 1px solid rgba(0, 0, 0, .1);
  color: rgba(0, 0, 0, .4);
}

.line_status_orange {
  font-weight: bold;
  display: inline-block;
  background: rgba(255, 61, 0, .7);
  padding: 10px;
  border-radius: 6px;
  border: 1px solid rgba(0, 0, 0, .1);
  color: rgba(0, 0, 0, .4);
}

.line_status_gray {
  font-weight: bold;
  display: inline-block;
  background: var(--gray-status-color);
  padding: 10px;
  border-radius: 6px;
  border: 1px solid rgba(0, 0, 0, .1);
  color: rgba(0, 0, 0, .4);
}

.clipboard {
  margin: 2px;
  opacity: 0.5;
  filter: invert(50%);
}

.clipboard:hover {
  opacity: 0.8;
  cursor: pointer;
}

.id_cont {
  display: inline;
  padding: 8px;
  border-radius: 6px;
  background: var(--checkbox);
}

table {
  border-collapse: collapse;
}

#job_info td {
  padding: 15px;
  border-bottom: 1px solid rgba(0, 0, 0, .05);
}

#job_info th,
#job_info td:nth-child(1) {
  font-weight: bold;
  font-size: 16pt;
  border-right: 1px solid rgba(0, 0, 0, .05);
}

#job_info th {
  text-align: left;
  font-size: 20pt;
  padding: 15px;
  border: 0;
}

#job_info tr:last-child td {
  border: 0px;
  border-right: 1px solid rgba(0, 0, 0, .05);
}

#job_info tr:last-child td:last-child {
  border: 0px;
}

#job_list td {
  padding: 15px;
  border-bottom: 1px solid rgba(0, 0, 0, .05);
}

#job_list th,
#job_list td:nth-child(1) {
  font-weight: bold;
}

#job_list th {
  text-align: left;
  font-size: 20pt;
  padding: 15px;
  border: 0;
}

#job_list tr:last-child td {
  border: 0px;
}

#job_list tr:last-child td:last-child {
  border: 0px;
}

.clipboard_toast {
  background: rgba(255, 61, 0, .8);
  color: rgba(0, 0, 0, .4);
  border-radius: 6px;
  top: 0;
  right: 0;
  margin-right: 15px;
  margin-top: 15px;
  position: fixed;
  display: flex;
  flex-direction: row;
  align-items: center;
  padding: 15px;
  gap: 10px;
}

.close_toast {
  font-weight: bold;
  cursor: pointer;
  margin-top: -2px;
}

.close_toast:hover {
  color: white;
}

.switch {
  position: relative;
  display: inline-block;
  width: 45px;
  height: 22px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(237, 233, 157, .1);
  border-radius: 6px;
}

.slider:before {
  position: absolute;
  content: "";
  height: 22px;
  width: 22px;
  margin: auto 0;
  background: silver;
  box-shadow: 0 0 3px 2px rgba(0, 0, 0, .1) inset;
  border-radius: 6px;
}

.switch input:checked + .slider {
  background: rgba(0, 100, 150, .1);
}

.switch input:checked + .slider:before {
  content: "";
  transform: translateX(24px);
  background: var(--light-orange);
  box-shadow: 0 0 3px 2px rgba(255, 0, 0, .1) inset;
}

#barContainer {
  width: 100%;
  padding: 0;
  margin: 0;
}

#progressBar {
  height: 5px;
  background-color: #ddd;
}

#barStatus {
  width: 0%;
  height: 100%;
  background-color: var(--dark-orange);
}

.reload {
  position: relative;
  display: inline-block;
  width: 20px;
  height: 20px;
}

.reload input {
  opacity: 0;
  width: 0;
  height: 0;
}

.check {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--checkbox);
  border-radius: 4px;
}

.check:before {
  position: absolute;
  content: "";
  height: 12px;
  width: 12px;
  margin: auto 0;
  border-radius: 2px;
  top: 4px;
  left: 4px;
}

.reload input:checked + .check:before {
  background: var(--light-orange);
  box-shadow: 0 0 3px 2px rgba(255, 0, 0, .1) inset;
}

::-webkit-scrollbar {
  width: 10px;
}

::-webkit-scrollbar-track {
  background: transparent;
  border-radius: 25px;
}

::-webkit-scrollbar-thumb {
  background: #888;
  border-radius: 25px;
}

::-webkit-scrollbar-thumb:hover {
  background: #555;
}

.repo{
  display: flex;
  gap: 6px;
  float: right;
  color: var(--font-color)!important;
}

.repo:hover{
  color: var(--dark-orange)!important;
}

.repo:active{
  color: var(--light-orange)!important;
}"#
};

// templates folder data: templates/cronframe.js
const CRONFRAME_JS: &str = {
    r#"// base template scripts

let stopModal = new tingle.modal({
    footer: true,
    stickyFooter: false,
    closeMethods: ['overlay', 'button', 'escape'],
    closeLabel: "Close",
    cssClass: ['custom-class-1', 'custom-class-2'],
    onOpen: function() {
        console.log('modal open');
    },
    onClose: function() {
        console.log('modal closed');
    },
    beforeClose: function() {
        return true; // close the modal
        return false; // nothing happens
    }
});

stopModal.setContent('<h1>Do you want to stop the Scheduler?</h1>');

stopModal.addFooterBtn('No', 'tingle-btn tingle-btn--pull-right tingle-btn', () => {
    stopModal.close();
});

stopModal.addFooterBtn('Yes', 'tingle-btn tingle-btn--pull-right tingle-btn--danger', () => {
    const url = window.location.href + "/stop_scheduler";
    console.log("request to: " + url);
    const xhr = new XMLHttpRequest();
    xhr.open("GET", url);
    xhr.send();
    xhr.responseType = "json";
    xhr.onload = () => {
        if (xhr.readyState == 4 && xhr.status == 200) {
            console.log(xhr.response);
            location.reload();
        } else {
            console.log(`Error: ${xhr.status}`);
        }
    };
});

let startModal = new tingle.modal({
    footer: true,
    stickyFooter: false,
    closeMethods: ['overlay', 'button', 'escape'],
    closeLabel: "Close",
    cssClass: ['custom-class-1', 'custom-class-2'],
    onOpen: function() {
        console.log('modal open');
    },
    onClose: function() {
        console.log('modal closed');
    },
    beforeClose: function() {
        // here's goes some logic
        // e.g. save content before closing the modal
        return true; // close the modal
        return false; // nothing happens
    }
});

startModal.setContent('<h1>Do you want to start the Scheduler?</h1>');

startModal.addFooterBtn('No', 'tingle-btn tingle-btn--pull-right tingle-btn', () => {
    startModal.close();
});

startModal.addFooterBtn('Yes', 'tingle-btn tingle-btn--pull-right tingle-btn--danger', () => {
    const url = window.location.href + "/start_scheduler";
    console.log("request to: " + url);
    const xhr = new XMLHttpRequest();
    xhr.open("GET", url);
    xhr.send();
    xhr.responseType = "json";
    xhr.onload = () => {
        if (xhr.readyState == 4 && xhr.status == 200) {
            console.log(xhr.response);
            location.reload();
        } else {
            console.log(`Error: ${xhr.status}`);
        }
    };
});

let barWidth = 0;

document.getElementById("barContainer").style.width = document.getElementById("container").style.width;

const advanceBar = () => {
    if (barWidth < 100) {
        barWidth = barWidth + 3.125;
        document.getElementById("barStatus").style.width = barWidth + '%';
    }
};

const reloadPage = () => {
    location.reload();
};

const setTheme = (value) => {
    localStorage.setItem('mode', value);
    document.documentElement.className = value;
};

const setAutoreload = (value) => {
    localStorage.setItem('autoreload', value);
    document.documentElement.className = value;
    reloadPage();
};

const toggleMode = () => {
    if (localStorage.getItem('mode') === 'dark-mode') {
        setTheme('light-mode');
    } else {
        setTheme('dark-mode');
    }
};

const toggleReload = () => {
    if (localStorage.getItem('autoreload') === 'yes') {
        setAutoreload('no');
    } else {
        setAutoreload('yes');
    }
};

const init = () => {
    setupTheme();
    setupBar();
};

const setupTheme = () => {
    if (localStorage.getItem('mode') === 'dark-mode') {
        setTheme('dark-mode');
        document.getElementById('slider').checked = false;
    } else {
        setTheme('light-mode');
        document.getElementById('slider').checked = true;
    }
};

const setupBar = () => {
    if (localStorage.getItem('autoreload') === 'yes') {
        setInterval(reloadPage, 5000);
        setInterval(advanceBar, 125);
        document.getElementById('autoreload').checked = true;
    } else {
        document.getElementById("barStatus").style.width = '100%';
        document.getElementById('autoreload').checked = false;
    }
};

init();

const startScheduler = () => {
    startModal.open();
}

const stopScheduler = () => {
    stopModal.open();
}

// job page scripts

let timeout = 0;
    let schedule = "* * * * * * *";

    const setTimeout = (value) => {
        console.log(value);
        timeout = value
    };

    const updateTimeout = () => {
        console.log("request to: " + window.location.href + "/toutset/" + timeout);
        const xhr = new XMLHttpRequest();
        xhr.open("GET", window.location.href + "/toutset/" + timeout);
        xhr.send();
        xhr.responseType = "json";
        xhr.onload = () => {
            if (xhr.readyState == 4 && xhr.status == 200) {
                console.log(xhr.response);
                location.reload();
            } else {
                console.log(`Error: ${xhr.status}`);
            }
        };
    }

    const setSchedule = (value) => {
        console.log(value);
        schedule = value
    };

    const updateSchedule = () => {
        console.log("request to: " + window.location.href + "/schedset/" + schedule);
        const xhr = new XMLHttpRequest();
        xhr.open("GET", window.location.href + "/schedset/" + schedule.replace("/", "slh"));
        xhr.send();
        xhr.responseType = "json";
        xhr.onload = () => {
            if (xhr.readyState == 4 && xhr.status == 200) {
                console.log(xhr.response);
                location.reload();
            } else {
                console.log(`Error: ${xhr.status}`);
            }
        };
    }

    const suspensionHandle = () => {
        console.log("request to: " + window.location.href + "/suspension_toggle");
        const xhr = new XMLHttpRequest();
        xhr.open("GET", window.location.href + "/suspension_toggle");
        xhr.send();
        xhr.responseType = "json";
        xhr.onload = () => {
            if (xhr.readyState == 4 && xhr.status == 200) {
                console.log(xhr.response);
                location.reload();
            } else {
                console.log(`Error: ${xhr.status}`);
            }
        };
    }

    const copyToClipBoard = (element) => {
        var copyText = document.getElementById(element);
        navigator.clipboard.writeText(copyText.innerHTML);
        toast("Copied to Clipboard");
    }

    let notify_shown = false;

    const toast = (text) => {
        if (notify_shown) return;

        notify_shown = true;

        var toast = document.createElement('div');
        toast.className = "clipboard_toast";

        var message = document.createElement("div");
        message.textContent = text;
        toast.appendChild(message);

        var close = document.createElement("div");
        close.className = "close_toast";
        close.innerHTML = "x"
        close.addEventListener("click", () => {
            toast.remove();
            notify_shown = false;
        })
        toast.append(close);

        document.body.appendChild(toast);

        window.setTimeout(() => {
            toast.remove();
            notify_shown = false;
        }, 3000);
    }"#
};

// templates folder data: templates/tingle.js
const TINGLE_JS: &str = {
    r#"/**
 * tingle.js - A simple modal plugin written in pure JavaScript
 * @version v0.16.0
 * @link https://github.com/robinparisi/tingle#readme
 * @license MIT
 */
 
/* global define, module */
(function (root, factory) {
  if (typeof define === 'function' && define.amd) {
    define(factory)
  } else if (typeof exports === 'object') {
    module.exports = factory()
  } else {
    root.tingle = factory()
  }
}(this, function () {
  /* ----------------------------------------------------------- */
  /* == modal */
  /* ----------------------------------------------------------- */

  var isBusy = false

  function Modal (options) {
    var defaults = {
      onClose: null,
      onOpen: null,
      beforeOpen: null,
      beforeClose: null,
      stickyFooter: false,
      footer: false,
      cssClass: [],
      closeLabel: 'Close',
      closeMethods: ['overlay', 'button', 'escape']
    }

    // extends config
    this.opts = extend({}, defaults, options)

    // init modal
    this.init()
  }

  Modal.prototype.init = function () {
    if (this.modal) {
      return
    }

    _build.call(this)
    _bindEvents.call(this)

    // insert modal in dom
    document.body.appendChild(this.modal, document.body.firstChild)

    if (this.opts.footer) {
      this.addFooter()
    }

    return this
  }

  Modal.prototype._busy = function (state) {
    isBusy = state
  }

  Modal.prototype._isBusy = function () {
    return isBusy
  }

  Modal.prototype.destroy = function () {
    if (this.modal === null) {
      return
    }

    // restore scrolling
    if (this.isOpen()) {
      this.close(true)
    }

    // unbind all events
    _unbindEvents.call(this)

    // remove modal from dom
    this.modal.parentNode.removeChild(this.modal)

    this.modal = null
  }

  Modal.prototype.isOpen = function () {
    return !!this.modal.classList.contains('tingle-modal--visible')
  }

  Modal.prototype.open = function () {
    if (this._isBusy()) return
    this._busy(true)

    var self = this

    // before open callback
    if (typeof self.opts.beforeOpen === 'function') {
      self.opts.beforeOpen()
    }

    if (this.modal.style.removeProperty) {
      this.modal.style.removeProperty('display')
    } else {
      this.modal.style.removeAttribute('display')
    }

    // prevent text selection when opening multiple times
    document.getSelection().removeAllRanges()

    // prevent double scroll
    this._scrollPosition = window.pageYOffset
    document.body.classList.add('tingle-enabled')
    document.body.style.top = -this._scrollPosition + 'px'

    // sticky footer
    this.setStickyFooter(this.opts.stickyFooter)

    // show modal
    this.modal.classList.add('tingle-modal--visible')

    // onOpen callback
    if (typeof self.opts.onOpen === 'function') {
      self.opts.onOpen.call(self)
    }

    self._busy(false)

    // check if modal is bigger than screen height
    this.checkOverflow()

    return this
  }

  Modal.prototype.close = function (force) {
    if (this._isBusy()) return
    this._busy(true)
    force = force || false

    //  before close
    if (typeof this.opts.beforeClose === 'function') {
      var close = this.opts.beforeClose.call(this)
      if (!close) {
        this._busy(false)
        return
      }
    }

    document.body.classList.remove('tingle-enabled')
    document.body.style.top = null
    window.scrollTo({
      top: this._scrollPosition,
      behavior: 'instant'
    })

    this.modal.classList.remove('tingle-modal--visible')

    // using similar setup as onOpen
    var self = this

    self.modal.style.display = 'none'

    // onClose callback
    if (typeof self.opts.onClose === 'function') {
      self.opts.onClose.call(this)
    }

    // release modal
    self._busy(false)
  }

  Modal.prototype.setContent = function (content) {
    // check type of content : String or Node
    if (typeof content === 'string') {
      this.modalBoxContent.innerHTML = content
    } else {
      this.modalBoxContent.innerHTML = ''
      this.modalBoxContent.appendChild(content)
    }

    if (this.isOpen()) {
      // check if modal is bigger than screen height
      this.checkOverflow()
    }

    return this
  }

  Modal.prototype.getContent = function () {
    return this.modalBoxContent
  }

  Modal.prototype.addFooter = function () {
    // add footer to modal
    _buildFooter.call(this)

    return this
  }

  Modal.prototype.setFooterContent = function (content) {
    // set footer content
    this.modalBoxFooter.innerHTML = content

    return this
  }

  Modal.prototype.getFooterContent = function () {
    return this.modalBoxFooter
  }

  Modal.prototype.setStickyFooter = function (isSticky) {
    // if the modal is smaller than the viewport height, we don't need sticky
    if (!this.isOverflow()) {
      isSticky = false
    }

    if (isSticky) {
      if (this.modalBox.contains(this.modalBoxFooter)) {
        this.modalBox.removeChild(this.modalBoxFooter)
        this.modal.appendChild(this.modalBoxFooter)
        this.modalBoxFooter.classList.add('tingle-modal-box__footer--sticky')
        _recalculateFooterPosition.call(this)
      }
      this.modalBoxContent.style['padding-bottom'] = this.modalBoxFooter.clientHeight + 20 + 'px'
    } else if (this.modalBoxFooter) {
      if (!this.modalBox.contains(this.modalBoxFooter)) {
        this.modal.removeChild(this.modalBoxFooter)
        this.modalBox.appendChild(this.modalBoxFooter)
        this.modalBoxFooter.style.width = 'auto'
        this.modalBoxFooter.style.left = ''
        this.modalBoxContent.style['padding-bottom'] = ''
        this.modalBoxFooter.classList.remove('tingle-modal-box__footer--sticky')
      }
    }

    return this
  }

  Modal.prototype.addFooterBtn = function (label, cssClass, callback) {
    var btn = document.createElement('button')

    // set label
    btn.innerHTML = label

    // bind callback
    btn.addEventListener('click', callback)

    if (typeof cssClass === 'string' && cssClass.length) {
      // add classes to btn
      cssClass.split(' ').forEach(function (item) {
        btn.classList.add(item)
      })
    }

    this.modalBoxFooter.appendChild(btn)

    return btn
  }

  Modal.prototype.resize = function () {
    // eslint-disable-next-line no-console
    console.warn('Resize is deprecated and will be removed in version 1.0')
  }

  Modal.prototype.isOverflow = function () {
    var viewportHeight = window.innerHeight
    var modalHeight = this.modalBox.clientHeight

    return modalHeight >= viewportHeight
  }

  Modal.prototype.checkOverflow = function () {
    // only if the modal is currently shown
    if (this.modal.classList.contains('tingle-modal--visible')) {
      if (this.isOverflow()) {
        this.modal.classList.add('tingle-modal--overflow')
      } else {
        this.modal.classList.remove('tingle-modal--overflow')
      }

      if (!this.isOverflow() && this.opts.stickyFooter) {
        this.setStickyFooter(false)
      } else if (this.isOverflow() && this.opts.stickyFooter) {
        _recalculateFooterPosition.call(this)
        this.setStickyFooter(true)
      }
    }
  }

  /* ----------------------------------------------------------- */
  /* == private methods */
  /* ----------------------------------------------------------- */

  function closeIcon () {
    return '<svg viewBox="0 0 10 10" xmlns="http://www.w3.org/2000/svg"><path d="M.3 9.7c.2.2.4.3.7.3.3 0 .5-.1.7-.3L5 6.4l3.3 3.3c.2.2.5.3.7.3.2 0 .5-.1.7-.3.4-.4.4-1 0-1.4L6.4 5l3.3-3.3c.4-.4.4-1 0-1.4-.4-.4-1-.4-1.4 0L5 3.6 1.7.3C1.3-.1.7-.1.3.3c-.4.4-.4 1 0 1.4L3.6 5 .3 8.3c-.4.4-.4 1 0 1.4z" fill="black" fill-rule="nonzero"/></svg>'
  }

  function _recalculateFooterPosition () {
    if (!this.modalBoxFooter) {
      return
    }
    this.modalBoxFooter.style.width = this.modalBox.clientWidth + 'px'
    this.modalBoxFooter.style.left = this.modalBox.offsetLeft + 'px'
  }

  function _build () {
    // wrapper
    this.modal = document.createElement('div')
    this.modal.classList.add('tingle-modal')

    // remove cusor if no overlay close method
    if (this.opts.closeMethods.length === 0 || this.opts.closeMethods.indexOf('overlay') === -1) {
      this.modal.classList.add('tingle-modal--noOverlayClose')
    }

    this.modal.style.display = 'none'

    // custom class
    this.opts.cssClass.forEach(function (item) {
      if (typeof item === 'string') {
        this.modal.classList.add(item)
      }
    }, this)

    // close btn
    if (this.opts.closeMethods.indexOf('button') !== -1) {
      this.modalCloseBtn = document.createElement('button')
      this.modalCloseBtn.type = 'button'
      this.modalCloseBtn.classList.add('tingle-modal__close')

      this.modalCloseBtnIcon = document.createElement('span')
      this.modalCloseBtnIcon.classList.add('tingle-modal__closeIcon')
      this.modalCloseBtnIcon.innerHTML = closeIcon()

      this.modalCloseBtnLabel = document.createElement('span')
      this.modalCloseBtnLabel.classList.add('tingle-modal__closeLabel')
      this.modalCloseBtnLabel.innerHTML = this.opts.closeLabel

      this.modalCloseBtn.appendChild(this.modalCloseBtnIcon)
      this.modalCloseBtn.appendChild(this.modalCloseBtnLabel)
    }

    // modal
    this.modalBox = document.createElement('div')
    this.modalBox.classList.add('tingle-modal-box')

    // modal box content
    this.modalBoxContent = document.createElement('div')
    this.modalBoxContent.classList.add('tingle-modal-box__content')

    this.modalBox.appendChild(this.modalBoxContent)

    if (this.opts.closeMethods.indexOf('button') !== -1) {
      this.modal.appendChild(this.modalCloseBtn)
    }

    this.modal.appendChild(this.modalBox)
  }

  function _buildFooter () {
    this.modalBoxFooter = document.createElement('div')
    this.modalBoxFooter.classList.add('tingle-modal-box__footer')
    this.modalBox.appendChild(this.modalBoxFooter)
  }

  function _bindEvents () {
    this._events = {
      clickCloseBtn: this.close.bind(this),
      clickOverlay: _handleClickOutside.bind(this),
      resize: this.checkOverflow.bind(this),
      keyboardNav: _handleKeyboardNav.bind(this)
    }

    if (this.opts.closeMethods.indexOf('button') !== -1) {
      this.modalCloseBtn.addEventListener('click', this._events.clickCloseBtn)
    }

    this.modal.addEventListener('mousedown', this._events.clickOverlay)
    window.addEventListener('resize', this._events.resize)
    document.addEventListener('keydown', this._events.keyboardNav)
  }

  function _handleKeyboardNav (event) {
    // escape key
    if (this.opts.closeMethods.indexOf('escape') !== -1 && event.which === 27 && this.isOpen()) {
      this.close()
    }
  }

  function _handleClickOutside (event) {
    // on macOS, click on scrollbar (hidden mode) will trigger close event so we need to bypass this behavior by detecting scrollbar mode
    var scrollbarWidth = this.modal.offsetWidth - this.modal.clientWidth
    var clickedOnScrollbar = event.clientX >= this.modal.offsetWidth - 15 // 15px is macOS scrollbar default width
    var isScrollable = this.modal.scrollHeight !== this.modal.offsetHeight
    if (navigator.platform === 'MacIntel' && scrollbarWidth === 0 && clickedOnScrollbar && isScrollable) {
      return
    }

    // if click is outside the modal
    if (this.opts.closeMethods.indexOf('overlay') !== -1 && !_findAncestor(event.target, 'tingle-modal') &&
        event.clientX < this.modal.clientWidth) {
      this.close()
    }
  }

  function _findAncestor (el, cls) {
    while ((el = el.parentElement) && !el.classList.contains(cls));
    return el
  }

  function _unbindEvents () {
    if (this.opts.closeMethods.indexOf('button') !== -1) {
      this.modalCloseBtn.removeEventListener('click', this._events.clickCloseBtn)
    }
    this.modal.removeEventListener('mousedown', this._events.clickOverlay)
    window.removeEventListener('resize', this._events.resize)
    document.removeEventListener('keydown', this._events.keyboardNav)
  }

  /* ----------------------------------------------------------- */
  /* == helpers */
  /* ----------------------------------------------------------- */

  function extend () {
    for (var i = 1; i < arguments.length; i++) {
      for (var key in arguments[i]) {
        if (arguments[i].hasOwnProperty(key)) {
          arguments[0][key] = arguments[i][key]
        }
      }
    }
    return arguments[0]
  }

  /* ----------------------------------------------------------- */
  /* == return */
  /* ----------------------------------------------------------- */

  return {
    modal: Modal
  }
}))
"#
};

// templates folder data: templates/tingle.css
const TINGLE_STYLES: &str = {
    r#"/**
 * tingle.js - A simple modal plugin written in pure JavaScript
 * @version v0.16.0
 * @link https://github.com/robinparisi/tingle#readme
 * @license MIT
 */

// modified for cronframe

.tingle-modal * {
  box-sizing: border-box;
}

.tingle-modal {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 1000;
  display: flex;
  visibility: hidden;
  flex-direction: column;
  align-items: center;
  overflow: hidden;
  -webkit-overflow-scrolling: touch;
  background: rgba(0, 0, 0, .9);
  opacity: 0;
  cursor: url("data:image/svg+xml,%3Csvg width='19' height='19' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M15.514.535l-6.42 6.42L2.677.536a1.517 1.517 0 00-2.14 0 1.517 1.517 0 000 2.14l6.42 6.419-6.42 6.419a1.517 1.517 0 000 2.14 1.517 1.517 0 002.14 0l6.419-6.42 6.419 6.42a1.517 1.517 0 002.14 0 1.517 1.517 0 000-2.14l-6.42-6.42 6.42-6.418a1.517 1.517 0 000-2.14 1.516 1.516 0 00-2.14 0z' fill='%23FFF' fill-rule='nonzero'/%3E%3C/svg%3E"), auto;
}

@supports ((-webkit-backdrop-filter: blur(12px)) or (backdrop-filter: blur(12px))) {
  .tingle-modal {
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
  }
}

/* confirm and alerts
-------------------------------------------------------------- */

.tingle-modal--confirm .tingle-modal-box {
  text-align: center;
}

/* modal
-------------------------------------------------------------- */

.tingle-modal--noOverlayClose {
  cursor: default;
}

.tingle-modal--noClose .tingle-modal__close {
  display: none;
}

.tingle-modal__close {
  position: fixed;
  top: 2.5rem;
  right: 2.5rem;
  z-index: 1000;
  padding: 0;
  width: 2rem;
  height: 2rem;
  border: none;
  background-color: transparent;
  color: #fff;
  cursor: pointer;
}

.tingle-modal__close svg * {
  fill: currentColor;
}

.tingle-modal__closeLabel {
  display: none;
}

.tingle-modal__close:hover {
  color: var(--light-orange);
  background: transparent;
}

.tingle-modal__close:active {
  color: var(--dark-orange);
  background: transparent;
}

.tingle-modal-box {
  max-width: 600px;
  position: relative;
  flex-shrink: 0;
  margin-top: auto;
  margin-bottom: auto;
  width: 60%;
  border-radius: 4px;
  background: var(--container-bg);
  opacity: 1;
  cursor: auto;
  will-change: transform, opacity;
}

.tingle-modal-box__content {
  padding: 3rem 3rem;
}

.tingle-modal-box__footer {
  padding: 1.5rem 2rem;
  width: auto;
  border-bottom-right-radius: 4px;
  border-bottom-left-radius: 4px;
  background-color: var(--content-bg);
  cursor: auto;
}

.tingle-modal-box__footer::after {
  display: table;
  clear: both;
  content: "";
}

.tingle-modal-box__footer--sticky {
  position: fixed;
  bottom: -200px; /* TODO : find a better way */
  z-index: 10001;
  opacity: 1;
  transition: bottom .3s ease-in-out .3s;
}

/* state
-------------------------------------------------------------- */

.tingle-enabled {
  position: fixed;
  right: 0;
  left: 0;
  overflow: hidden;
}

.tingle-modal--visible .tingle-modal-box__footer {
  bottom: 0;
}

.tingle-modal--visible {
  visibility: visible;
  opacity: 1;
}

.tingle-modal--visible .tingle-modal-box {
  animation: scale .2s cubic-bezier(.68, -.55, .265, 1.55) forwards;
}

.tingle-modal--overflow {
  overflow-y: scroll;
  padding-top: 8vh;
}

/* btn
-------------------------------------------------------------- */

.tingle-btn {
  display: inline-block;
  margin: 0 .5rem;
  padding: 1rem 2rem;
  border: none;
  background-color: grey;
  box-shadow: none;
  color: #fff;
  vertical-align: middle;
  text-decoration: none;
  font-size: inherit;
  font-family: inherit;
  line-height: normal;
  cursor: pointer;
  transition: background-color .4s ease;
}

.tingle-btn--primary {
  background-color: #3498db;
}

.tingle-btn--danger {
  background-color: var(--light-orange);
}

.tingle-btn--default {
  background-color: #34495e;
}

.tingle-btn--pull-left {
  float: left;
}

.tingle-btn--pull-right {
  float: right;
}

/* responsive
-------------------------------------------------------------- */

@media (max-width : 540px) {
  .tingle-modal {
    top: 0px;
    display: block;
    padding-top: 60px;
    width: 100%;
  }

  .tingle-modal-box {
    width: auto;
    border-radius: 0;
  }

  .tingle-modal-box__content {
    overflow-y: scroll;
  }

  .tingle-modal--noClose {
    top: 0;
  }

  .tingle-modal--noOverlayClose {
    padding-top: 0;
  }

  .tingle-modal-box__footer .tingle-btn {
    display: block;
    float: none;
    margin-bottom: 1rem;
    width: 100%;
  }

  .tingle-modal__close {
    top: 0;
    right: 0;
    left: 0;
    display: block;
    width: 100%;
    height: 60px;
    border: none;
    background-color: var(--content-bg);
    box-shadow: none;
    color: #fff;
  }

  .tingle-modal__closeLabel {
    display: inline-block;
    vertical-align: middle;
    font-size: 1.6rem;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Cantarell", "Fira Sans", "Droid Sans", "Helvetica Neue", sans-serif;
  }

  .tingle-modal__closeIcon {
    display: inline-block;
    margin-right: .8rem;
    width: 1.6rem;
    vertical-align: middle;
    font-size: 0;
  }
}

/* animations
-------------------------------------------------------------- */

@keyframes scale {
  0% {
    opacity: 0;
    transform: scale(.9);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}
"#
};
