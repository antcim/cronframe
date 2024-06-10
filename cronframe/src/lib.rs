#[macro_use]
extern crate rocket;
pub use chrono::{DateTime, Duration, Utc};
pub use cron::Schedule;
pub use cronframe_macro::{cron, cron_impl, cron_obj, job};
use crossbeam_channel::{Receiver, Sender};
pub use linkme::distributed_slice;
pub use log::{info, warn, LevelFilter};
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};
use rand::distributions::DistString;
pub use std::{
    any::{self, Any, TypeId},
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
    thread::JoinHandle,
    vec,
};

mod server;

// necessary to gather all the annotated jobs automatically
inventory::collect!(JobBuilder<'static>);

const ID_SIZE: usize = 8;

pub enum JobBuilder<'a> {
    Global {
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    },
    Method {
        name: &'a str,
        job: fn(arg: Arc<Box<dyn Any + Send + Sync>>),
        cron_expr: String,
        timeout: String,
        instance: Arc<Box<dyn Any + Send + Sync>>
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
        JobBuilder::Global {
            name,
            job,
            cron_expr,
            timeout,
        }
    }

    pub const fn method_job(
        name: &'a str,
        job: fn(arg: Arc<Box<dyn Any + Send + Sync>>),
        cron_expr: String,
        timeout: String,
        instance: Arc<Box<dyn Any + Send + Sync>>
    ) -> Self {
        JobBuilder::Method {
            name,
            job,
            cron_expr,
            timeout,
            instance
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

    pub fn build(self) -> CronJob {
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
                    id: generate_id(ID_SIZE),
                    job: CronJobType::Global(job),
                    schedule,
                    timeout,
                    channels: None,
                    start_time: None,
                    run_id: None,
                    instance: None
                }
            }
            Self::Method {
                name,
                job,
                cron_expr,
                timeout,
                instance,
            } => {
                let schedule =
                    Schedule::from_str(&cron_expr).expect("Failed to parse cron expression!");
                let timeout: i64 = timeout.parse().expect("Failed to parse timeout!");
                let timeout = if timeout > 0 {
                    Some(Duration::milliseconds(timeout))
                } else {
                    None
                };

                CronJob {
                    name: name.to_string(),
                    id: generate_id(ID_SIZE),
                    job: CronJobType::Method(job),
                    schedule,
                    timeout,
                    channels: None,
                    start_time: None,
                    run_id: None,
                    instance: Some(instance)
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
                    id: generate_id(ID_SIZE),
                    job: CronJobType::Function(job),
                    schedule,
                    timeout,
                    channels: None,
                    start_time: None,
                    run_id: None,
                    instance: None
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CronJobType {
    Global(fn()),
    Method(fn(arg: Arc<Box<dyn Any + Send + Sync>>)), //maybe add object id here
    Function(fn()),
}

#[derive(Debug, Clone)]
pub struct CronJob {
    pub name: String,
    id: String,
    job: CronJobType,
    schedule: Schedule,
    timeout: Option<Duration>,
    channels: Option<(Sender<String>, Receiver<String>)>,
    start_time: Option<DateTime<Utc>>,
    run_id: Option<String>,
    instance: Option<Arc<Box<dyn Any + Send + Sync>>>,
}

impl CronJob {
    pub fn try_schedule(&mut self) -> Option<JoinHandle<()>> {
        if self.check_schedule() {
            self.run_id = Some(generate_id(ID_SIZE));
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

    pub fn set_timeout(&mut self, value: i64) {
        self.timeout = if value > 0 {
            Some(Duration::milliseconds(value))
        } else {
            None
        };
    }

    pub fn status(&self) -> String {
        if self.check_timeout() {
            "Timed-Out".to_string()
        } else if self.run_id.is_some() {
            "Running".to_string()
        } else {
            "Awaiting Schedule".to_string()
        }
    }

    pub fn set_schedule(&mut self, expression: &str) -> bool {
        let expr = expression.replace("slh", "/").replace("%20", " ");
        if let Ok(schedule) = Schedule::from_str(expr.as_str()) {
            self.schedule = schedule;
            return true;
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

    pub fn schedule(&self) -> String {
        self.schedule.to_string()
    }

    pub fn upcoming(&self) -> String {
        if self.check_timeout() {
            return "None due to timeout.".to_string();
        }
        self.schedule
            .upcoming(Utc)
            .into_iter()
            .next()
            .unwrap()
            .to_string()
    }

    pub fn get_run_id(&self) -> String {
        match &self.run_id {
            Some(string) => string.clone(),
            None => "None".into(),
        }
    }

    pub fn run(&self) -> JoinHandle<()> {
        let tx = self.channels.as_ref().unwrap().0.clone();
        let _rx = self.channels.as_ref().unwrap().1.clone();
        let schedule = self.schedule.clone();
        let job_id = format!("{} ID#{}", self.name, self.id);
        let run_id = self.run_id.as_ref().unwrap().clone();
        
        match self.job {
            CronJobType::Method(job) => {
                let instance = self.instance.clone().unwrap();
                let job_thread = move || loop {
                    let now = Utc::now();
                    if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                        let until_next = next - now;
                        std::thread::sleep(until_next.to_std().unwrap());
                        info!("job @{job_id} RUN_ID#{run_id} - Execution");
                        job(instance);
                        let _ = tx.send("JOB_COMPLETE".to_string());
                        break;
                    }
                };
                std::thread::spawn(job_thread)
            }
            CronJobType::Global(job) | CronJobType::Function(job) => {
                let job_thread = move || loop {
                    let now = Utc::now();
                    if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                        let until_next = next - now;
                        std::thread::sleep(until_next.to_std().unwrap());
                        info!("job @{job_id} RUN_ID#{run_id} - Execution");
                        job();
                        let _ = tx.send("JOB_COMPLETE".to_string());
                        break;
                    }
                };
                std::thread::spawn(job_thread)
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

        std::thread::spawn(move || server::server(frame));
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
            std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());

            let mut cron_jobs = instance.cron_jobs.lock().unwrap();

            for cron_job in &mut (*cron_jobs) {
                let job_id = format!("{} ID#{}", cron_job.name, cron_job.id);

                // if the job_id key is not in the hashmap then attempt to schedule it
                // if scheduling is a succed then add the key to the hashmap
                if !instance.handlers.lock().unwrap().contains_key(&job_id) {
                    if cron_job.check_timeout() {
                        info!("job @{} - Reached Timeout", job_id);
                        // TODO remove timed-out job from list of actives?
                        continue;
                    }

                    let handle = (*cron_job).try_schedule();

                    if handle.is_some() {
                        instance
                            .handlers
                            .lock()
                            .unwrap()
                            .insert(job_id.clone(), handle.unwrap());
                        info!(
                            "job @{} RUN_ID#{} - Scheduled.",
                            job_id,
                            cron_job.run_id.as_ref().unwrap()
                        );
                    }
                }
                // the job_id key is in the hashmap and running
                // check to see if it sent a message that says it finished
                else {
                    let _tx = cron_job
                        .channels
                        .clone()
                        .expect("error: unwrapping tx channel")
                        .0;
                    let rx = cron_job
                        .channels
                        .clone()
                        .expect("error: unwrapping rx channel")
                        .1;

                    match rx.try_recv() {
                        Ok(message) => {
                            if message == "JOB_COMPLETE" {
                                info!(
                                    "job @{} RUN_ID#{} - Completed.",
                                    job_id,
                                    cron_job.run_id.as_ref().unwrap()
                                );
                                instance.handlers.lock().unwrap().remove(job_id.as_str());
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
        std::thread::spawn(scheduler);
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

fn generate_id(len: usize) -> String {
    rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}
