#[macro_use] extern crate rocket;
extern crate lazy_static;
use chrono::DateTime;
pub use chrono::{Duration, Utc};
pub use cron::Schedule;
pub use cronframe_macro::{cron, cron_impl, cron_obj, job};
use crossbeam_channel::{Receiver, Sender};
pub use lazy_static::lazy_static;
pub use linkme::distributed_slice;
pub use log::{info, warn, LevelFilter};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use rand::distributions::{Alphanumeric, DistString};
pub use std::any::Any;
pub use std::any::{self, TypeId};
pub use std::str::FromStr;
use std::sync::Arc;
pub use std::thread;
pub use std::{collections::HashMap, sync::Mutex, thread::JoinHandle, vec};

// necessary to gather all the annotated jobs automatically
inventory::collect!(JobBuilder<'static>);

pub enum JobBuilder<'a> {
    Global {
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    },
    Method {
        name: &'a str,
        job: fn(arg: &dyn Any),
        cron_expr: String,
        timeout: String,
        // add parameter to get a reference to self?
    },
    Function {
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    },
}

impl<'a> JobBuilder<'a> {
    pub const fn global_job(
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    ) -> Self {
        JobBuilder::Function {
            name,
            job,
            cron_expr,
            timeout,
        }
    }

    pub const fn method_job(
        name: &'a str,
        job: fn(arg: &dyn Any),
        cron_expr: String,
        timeout: String,
    ) -> Self {
        JobBuilder::Method {
            name,
            job,
            cron_expr,
            timeout,
        }
    }

    pub const fn function_job(
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    ) -> Self {
        JobBuilder::Function {
            name,
            job,
            cron_expr,
            timeout,
        }
    }

    pub fn build(&self) -> CronJob {
        match self {
            Self::Global {
                name,
                job,
                cron_expr,
                timeout,
            } => {
                let schedule =
                    Schedule::from_str(cron_expr).expect("Failed to parse cron expression!");
                let timeout: i64 = timeout.parse().expect("Failed to parse timeout!");
                let timeout = if timeout > 0 {
                    Some(Duration::milliseconds(timeout))
                } else {
                    None
                };

                CronJob {
                    name: name.to_string(),
                    job: CronJobType::Global(*job),
                    schedule,
                    timeout,
                    channels: None,
                    start_time: None,
                    run_id: None,
                }
            }
            Self::Method {
                name,
                job,
                cron_expr,
                timeout,
            } => {
                let schedule =
                    Schedule::from_str(cron_expr).expect("Failed to parse cron expression!");
                let timeout: i64 = timeout.parse().expect("Failed to parse timeout!");
                let timeout = if timeout > 0 {
                    Some(Duration::milliseconds(timeout))
                } else {
                    None
                };

                CronJob {
                    name: name.to_string(),
                    job: CronJobType::Method(*job),
                    schedule,
                    timeout,
                    channels: None,
                    start_time: None,
                    run_id: None,
                }
            }
            Self::Function {
                name,
                job,
                cron_expr,
                timeout,
            } => {
                let schedule =
                    Schedule::from_str(cron_expr).expect("Failed to parse cron expression!");
                let timeout: i64 = timeout.parse().expect("Failed to parse timeout!");
                let timeout = if timeout > 0 {
                    Some(Duration::milliseconds(timeout))
                } else {
                    None
                };

                CronJob {
                    name: name.to_string(),
                    job: CronJobType::Function(*job),
                    schedule,
                    timeout,
                    channels: None,
                    start_time: None,
                    run_id: None,
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CronJobType {
    Global(fn()),
    Method(fn(arg: &dyn Any)), //maybe add object id here
    Function(fn()),
}

#[derive(Debug, Clone)]
pub struct CronJob {
    pub name: String,
    job: CronJobType,
    schedule: Schedule,
    timeout: Option<Duration>,
    channels: Option<(Sender<String>, Receiver<String>)>,
    start_time: Option<DateTime<Utc>>,
    run_id: Option<String>,
    // add option parameter to get a reference to self in case of Method Job?
}

impl CronJob {
    pub fn try_schedule(&mut self) -> Option<JoinHandle<()>> {
        if self.check_schedule() {
            self.run_id = Some(Alphanumeric.sample_string(&mut rand::thread_rng(), 8));
            self.channels = Some(crossbeam_channel::bounded(1));
            if self.start_time.is_none() {
                self.start_time = Some(Utc::now());
            }
            return Some(self.run());
        }
        None
    }

    pub fn check_schedule(&self) -> bool {
        let now = Utc::now();
        if let Some(next) = self.schedule.upcoming(Utc).take(1).next() {
            let until_next = (next - now).num_milliseconds();
            if until_next <= 1000 {
                return true;
            }
        }
        false
    }

    pub fn check_timeout(&self) -> bool {
        if let Some(timeout) = self.timeout {
            if self.start_time.is_some() {
                let timeout = self.start_time.unwrap() + timeout;
                let now = Utc::now();
                if now >= timeout {
                    return true;
                }
            }
        }
        false
    }

    pub fn run(&self) -> JoinHandle<()> {
        let tx = self.channels.as_ref().unwrap().0.clone();
        let _rx = self.channels.as_ref().unwrap().1.clone();
        let schedule = self.schedule.clone();
        let job_name = self.name.clone();
        let run_id = self.run_id.as_ref().unwrap().clone();

        match self.job {
            CronJobType::Method(job) => {
                let job_thread = move || loop {
                    let now = Utc::now();
                    if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                        let until_next = next - now;
                        thread::sleep(until_next.to_std().unwrap());
                        info!("job @ {job_name} # {run_id} - Execution");
                        job(&());
                        let _ = tx.send("JOB_COMPLETE".to_string());
                        break;
                    }
                };
                thread::spawn(job_thread)
            }
            CronJobType::Global(job) | CronJobType::Function(job) => {
                let job_thread = move || loop {
                    let now = Utc::now();
                    if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                        let until_next = next - now;
                        thread::sleep(until_next.to_std().unwrap());
                        info!("job @ {job_name} # {run_id} - Execution");
                        job();
                        let _ = tx.send("JOB_COMPLETE".to_string());
                        break;
                    }
                };
                thread::spawn(job_thread)
            }
        }
    }
}
pub struct CronFrame {
    pub cron_jobs: Mutex<Vec<CronJob>>,
    handlers: Mutex<HashMap<String, JoinHandle<()>>>,
    _logger: log4rs::Handle,
}
impl CronFrame {
    pub fn init() -> Arc<CronFrame> {
        let _logger = Self::default_logger();
        let frame = CronFrame {
            cron_jobs: Mutex::new(vec![]),
            handlers: Mutex::new(HashMap::new()),
            _logger,
        };

        info!("CronFrame Init Start.");
        info!("Colleting Global Jobs.");
        // get the automatically collected global jobs
        for job_builder in inventory::iter::<JobBuilder> {
            let cron_job = job_builder.build();
            info!("Found Global Job \"{}\"", cron_job.name);
            frame.cron_jobs.lock().unwrap().push(cron_job)
        }
        info!("Global Jobs Collected.");
        info!("CronFrame Init Complete.");

        info!("CronFrame Server Init");
        let frame = Arc::new(frame);
        let ret_frame = frame.clone();
        // spawn the server thread
        std::thread::spawn(move || server(frame));
        info!("CronFrame Server Running...");
        ret_frame
    }

    pub fn add_job(&mut self, job: CronJob) {
        self.cron_jobs.lock().unwrap().push(job)
    }

    pub fn scheduler(self: &Arc<Self>) {
        let instance = self.clone();

        let scheduler = move || loop {
            // sleep some otherwise the cpu consumption goes to the moon
            thread::sleep(Duration::milliseconds(500).to_std().unwrap());

            let mut cron_jobs = instance.cron_jobs.lock().unwrap();

            for cron_job in &mut(*cron_jobs) {
                if !instance.handlers.lock().unwrap().contains_key(&cron_job.name) {
                    if cron_job.check_timeout() {
                        info!("job @ {} - Reached Timeout", cron_job.name);
                        // TODO remove timedout job from list of actives?
                        continue;
                    }
                    let handle = (*cron_job).try_schedule();
                    if handle.is_some() {
                        instance.handlers.lock().unwrap().insert(cron_job.name.clone(), handle.unwrap());
                        info!(
                            "job @ {} # {} - Scheduled.",
                            cron_job.name,
                            cron_job.run_id.as_ref().unwrap()
                        );
                    }
                } else {
                    let _tx = cron_job.channels.clone().expect("error: unwrapping tx channel").0;
                    let rx = cron_job.channels.clone().expect("error: unwrapping rx channel").1;

                    match rx.try_recv() {
                        Ok(message) => {
                            if message == "JOB_COMPLETE" {
                                info!(
                                    "job @ {} # {} - Completed.",
                                    cron_job.name,
                                    cron_job.run_id.as_ref().unwrap()
                                );
                                instance.handlers.lock().unwrap().remove(cron_job.name.as_str());
                                cron_job.channels = None;
                                cron_job.run_id = None;
                            }
                        }
                        Err(_error) => (),
                    }
                    cron_job.check_timeout();
                }
            }
        };
        thread::spawn(scheduler);
        info!("CronFrame Scheduler Running...");
    }

    fn default_logger() -> log4rs::Handle {
        let pattern = "{d(%Y-%m-%d %H:%M:%S UTC%Z)} {l} {t} - {m}{n}";

        let log_file = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern)))
            .append(false)
            .build("log/cronframe.log")
            .unwrap();

        let config = Config::builder()
            .appender(Appender::builder().build("log_file", Box::new(log_file)))
            .build(
                Root::builder()
                    .appender("log_file")
                    .build(LevelFilter::Info),
            )
            .unwrap();

        log4rs::init_config(config).unwrap()
    }
}

#[get("/")]
fn home(cronframe: &rocket::State<Arc<CronFrame>>) -> String{
    //cronframe.scheduler();
    //format!("{}", cronframe.cron_jobs[0].name)
    "running".to_string()
}

fn server(frame: Arc<CronFrame>) -> anyhow::Result<i32> {
    let tokio_runtime = rocket::tokio::runtime::Runtime::new()?;

    let config = rocket::Config {
        port: 8002,
        address: std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
        ..rocket::Config::debug_default()
    };

    let rocket = rocket::custom(&config).mount("/", routes![home]).manage(frame);
    
    tokio_runtime.block_on(async move {
        let _ = rocket.launch().await;
    });

    Ok(0)
}