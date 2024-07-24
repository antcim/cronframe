//! Custom setup of rocket.rs for the cronframe web server

use crate::{
    config::read_config,
    cronframe::{self, CronFrame},
    CronFilter, CronJobType,
};
use log::info;
use rocket::{config::Shutdown, figment::value::magic::RelativePathBuf, futures::FutureExt, serde::Serialize};
use rocket_dyn_templates::{context, Engines, Template};
use std::{fs, sync::Arc, time::Duration};

/// Called by the init funciton of the Cronframe type for setting up the web server
/// 
/// It provides 4 routes, two of which are API only.
/// 
/// Upon first start of the library it will generate a templates folder inside the current director with the following files:
/// - base.html.tera
/// - index.htm.tera
/// - job.html.tera
/// - styles.css
pub fn web_server(frame: Arc<CronFrame>) {
    if !std::path::Path::new("./templates").exists(){
        println!("Generating templates directory content...");
        fs::create_dir("templates").expect("could not create templates directory");
        fs::write(std::path::Path::new("./templates/base.html.tera"), BASE_TEMPLATE);
        fs::write(std::path::Path::new("./templates/index.html.tera"), INDEX_TEMPLATE);
        fs::write(std::path::Path::new("./templates/job.html.tera"), JOB_TEMPLATE);
        fs::write(std::path::Path::new("./templates/styles.css"), STYLES);
        std::thread::sleep(Duration::from_secs(5));
    }

    let cronframe = frame.clone();

    let tokio_runtime = rocket::tokio::runtime::Runtime::new().unwrap();

    let config = match read_config() {
        Some(config_data) => rocket::Config {
            port: {
                if let Some(webserver_data) = config_data.webserver{
                    webserver_data.port.unwrap_or_else(|| 8098)
                }else{
                    8098
                }
            },
            address: std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
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
            routes![styles, home, job_info, update_timeout, update_schedule],
        )
        .attach(Template::fairing())
        .manage(frame);

    let (tx, rx) = cronframe.web_server_channels.clone();

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
    upcoming_utc: String,
    upcoming_local: String,
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

const BASE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>

<head>
    <meta charset="utf-8" />
    <title>CronFrame</title>
    <link href='https://fonts.googleapis.com/css?family=Lato' rel='stylesheet'>
    <link rel="stylesheet" href="/styles">
</head>

<body>
    <div id="wrapper">
        <div id="container">
            <header>
                <div id="logo">
                    <a href="/"><span style="color:#494949">Cron</span><span style="color:#FF3D00">Frame</span></a>
                </div>
            </header>
            <div id="content">
                {% block content %}
                {% endblock content %}
            </div>
            <footer>
                &lt;/&gt; Antonio Cimino
            </footer>
        </div>
    </div>
    <script>
        const reloadPage = () => {
            location.reload();
        };
    </script>
</body>

</html>"#;

const INDEX_TEMPLATE: &str = r#"{% extends "base" %}

{% block content %}
<table id="job_list">
    <tr>
        <th>
            Current Jobs
            <div id="refresh" onclick="reloadPage()">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 25 25">
                    <path
                        d="M21,15v-5c0-3.866-3.134-7-7-7l-3,0c-0.552,0-1,0.448-1,1v0c0,1.657,1.343,3,3,3h1	c1.657,0,3,1.343,3,3v5h-1.294c-0.615,0-0.924,0.742-0.491,1.178l3.075,3.104c0.391,0.395,1.03,0.395,1.421,0l3.075-3.104	C23.218,15.742,22.908,15,22.294,15H21z"
                        opacity=".35"></path>
                    <path
                        d="M3,9v5c0,3.866,3.134,7,7,7h3c0.552,0,1-0.448,1-1v0c0-1.657-1.343-3-3-3h-1c-1.657,0-3-1.343-3-3V9h1.294	c0.615,0,0.924-0.742,0.491-1.178L5.71,4.717c-0.391-0.395-1.03-0.395-1.421,0L1.215,7.822C0.782,8.258,1.092,9,1.706,9H3z">
                    </path>
                </svg>
            </div>
        </th>
    </tr>
    {% for cron_job in cron_jobs %}
    {% set link = "/job/" ~ cron_job.name ~ "/" ~ cron_job.id %}
    <tr>
        <td><a href="{{link}}">{{cron_job.name}}</a></td>
        <td>{{cron_job.id}}</td>
    </tr>
    {% endfor %}
</table>
{% endblock content %}"#;

const JOB_TEMPLATE: &str = r#"{% extends "base" %}

{% block content %}

{% if job_info.name != ""%}
<table id="job_info">
    <tr>
        <th colspan="2">
            Job Info @{{job_info.name}}
            <div id="refresh" onclick="reloadPage()">
                <svg xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" viewBox="0 0 24 24">
                    <path
                        d="M21,15v-5c0-3.866-3.134-7-7-7l-3,0c-0.552,0-1,0.448-1,1v0c0,1.657,1.343,3,3,3h1	c1.657,0,3,1.343,3,3v5h-1.294c-0.615,0-0.924,0.742-0.491,1.178l3.075,3.104c0.391,0.395,1.03,0.395,1.421,0l3.075-3.104	C23.218,15.742,22.908,15,22.294,15H21z"
                        opacity=".35"></path>
                    <path
                        d="M3,9v5c0,3.866,3.134,7,7,7h3c0.552,0,1-0.448,1-1v0c0-1.657-1.343-3-3-3h-1c-1.657,0-3-1.343-3-3V9h1.294	c0.615,0,0.924-0.742,0.491-1.178L5.71,4.717c-0.391-0.395-1.03-0.395-1.421,0L1.215,7.822C0.782,8.258,1.092,9,1.706,9H3z">
                    </path>
                </svg>
            </div>
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
        <td colspan="2">{{job_info.type}}</td>
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
        <td colspan="2">
            {% if job_info.status == "Timed-Out" %}
            <div class="line_status_gray">{{job_info.status}}</div>
            {% elif job_info.status == "Running" %}
            <div class="line_status_green">{{job_info.status}}</div>
            {% else %}
            <div class="line_status_yellow">{{job_info.status}}</div>
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
                <p>{{job_info.upcoming_local}} (Local)</p>
            {% endif %}
        </td>
    </tr>
</table>

<script>
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
    }
</script>

{% else %}
<div id="job_info">
    <div class="job_info_item">
        Job not found
    </div>
</div>
{% endif %}
{% endblock content %}"#;

const STYLES: &str = r#"body{
    background: #F6F6F6;
    color: #494949;
    font-family: "Lato"!important;
    margin: 0;
}

a{
    text-decoration: none;
    text-shadow: 1px 1px 1px rgba(255,255,255,1);
}

a:link {
    color: #FF3D00;
}

a:visited {
    color: #ffa702;
}

a:hover {
    color: green;
}

a:active {
    color: red;
}

header{
    display: flex;
    padding: 15px;
    align-items: center;
    justify-content: center;
}

#logo{
    flex: 2;
    font-weight: bold;
    font-size: 30pt;
}

#refresh{
    display: inline-block;
    hight: auto;
    width: 25px;
    opacity: 0.5;
}

#refresh:hover{
    cursor: pointer;
    opacity: 1;
}

footer{
    padding: 15px;
    text-align: right;
}

#wrapper{
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100vh;
    padding: 0;
    margin: 0;
}

#container{
    display: flex;
    flex-direction: column;
    gap: 5px;
    background: white;
    padding: 10px;
    padding-top: 5px;
    padding-bottomn: 5px;
    border--radius: 6px;
    box-shadow: 0px 0px 3px 0px rgba(0,0,0,.1);
    border-top: 4px solid #FF3D00;
    max-width: 1200px;
}

#status{
    font-weight: bold;
    background: rgba(51, 255, 0, .3);
    padding: 15px;
    border-radius: 6px;
    color: rgba(0,0,0,.4);
}

#content{
    background: #F1F1F1;
    padding: 15px;
    border-radius: 6px;
}

input[type=text], input[type=number] {
    padding: 10px;
    display: inline-block;
    border: 1px solid #ccc;
    border-radius: 4px;
    box-sizing: border-box;
}

input[type=text]:focus, input[type=number]:focus {
    border-color: rgba(229, 103, 23, 0.8);
    box-shadow: 0 1px 1px rgba(229, 103, 23, 0.075) inset, 0 0 4px rgba(229, 103, 23, 0.6);
    outline: 0 none;
}

button {
    background-color: rgba(0,0,0,.5);
    color: white;
    padding: 10px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
}

button:hover {
    background-color: #4CAF50;
    color: white;
    padding: 10px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
}

button:active {
    background-color: #FF3D00;
    color: white;
    padding: 10px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
}

.line_status_green{
    font-weight: bold;
    display: inline-block;
    background: rgba(51, 255, 0, .5);
    padding: 10px;
    border-radius: 6px;
    border: 1px solid rgba(0,0,0,.1);
    color: rgba(0,0,0,.4);
}

.line_status_yellow{
    font-weight: bold;
    display: inline-block;
    background: rgba(255, 236, 102, 1);
    padding: 10px;
    border-radius: 6px;
    border: 1px solid rgba(0,0,0,.1);
    color: rgba(0,0,0,.4);
}

.line_status_orange{
    font-weight: bold;
    display: inline-block;
    background: rgba(255, 61, 0, .8);
    padding: 10px;
    border-radius: 6px;
    border: 1px solid rgba(0,0,0,.1);
    color: rgba(0,0,0,.4);
}

.line_status_gray{
    font-weight: bold;
    display: inline-block;
    background: rgba(0, 0, 0, .3);
    padding: 10px;
    border-radius: 6px;
    border: 1px solid rgba(0,0,0,.1);
    color: rgba(0,0,0,.4);
}

.clipboard{
    margin: 2px;
    opacity: 0.5;
}

.clipboard:hover{
    opacity: 0.8;
    cursor: pointer;
}

.id_cont{
    display: inline;
    padding: 8px;
    border-radius: 6px;
    background: rgba(0,0,0,.1);
}

table{
    border-collapse: collapse;
}

#job_info td{
    padding: 15px;
    border-bottom: 1px solid rgba(0,0,0,.05)
}

#job_info th, #job_info td:nth-child(1){
    font-weight: bold;
    font-size: 16pt;
    border-right: 1px solid rgba(0,0,0,.05)
}

#job_info th{
    text-align: left;
    font-size: 20pt;
    padding: 15px;
    border: 0;
}

#job_info tr:last-child td{
    border: 0px;
    border-right: 1px solid rgba(0,0,0,.05)
}

#job_info tr:last-child td:last-child{
    border: 0px;
}

#job_list td{
    padding: 15px;
    border-bottom: 1px solid rgba(0,0,0,.05)
}

#job_list th, #job_list td:nth-child(1){
    font-weight: bold;
}

#job_list th{
    text-align: left;
    font-size: 20pt;
    padding: 15px;
    border: 0;
}

#job_list tr:last-child td{
    border: 0px;
}

#job_list tr:last-child td:last-child{
    border: 0px;
}

.clipboard_toast{
    background: rgba(255, 61, 0, .8);
    color: rgba(0,0,0,.4);
    border-radius: 6px;
    top:0;
    right: 0;
    margin-right: 15px;
    margin-top: 15px;
    position:fixed;
    display:flex;
    flex-direction:row;
    align-items: center;
    padding: 15px;
    gap: 10px;
}

.close_toast {
    font-weight: bold;
    cursor:pointer;
    margin-top: -2px;
}

.close_toast:hover {
    color: white;
}"#;