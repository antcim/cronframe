extern crate lazy_static;
use chrono::DateTime;
pub use chrono::{Duration, Utc};
pub use cron::Schedule;
pub use cronframe_macro::{cron, cron_impl, cron_obj, job};
use crossbeam_channel::{Receiver, Sender};
pub use lazy_static::lazy_static;
pub use log::{info, warn, LevelFilter};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Handle;
use rand::distributions::{Alphanumeric, DistString};
pub use std::any::Any;
pub use std::any::{self, TypeId};
pub use std::str::FromStr;
pub use std::thread;
pub use std::{collections::HashMap, sync::Mutex, thread::JoinHandle, vec};
pub use linkme::distributed_slice;

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
        job: fn(&dyn Any),
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
                    handler: None,
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
                    handler: None,
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
                    handler: None,
                    channels: None,
                    start_time: None,
                    run_id: None,
                }
            }
        }
    }
}

pub enum CronJobType {
    Global(fn()),
    Method(fn(_self: &dyn Any)),
    Function(fn()),
}

pub struct CronJob {
    pub name: String,
    job: CronJobType,
    schedule: Schedule,
    timeout: Option<Duration>,
    handler: Option<JoinHandle<()>>,
    channels: Option<(Sender<String>, Receiver<String>)>,
    start_time: Option<DateTime<Utc>>,
    run_id: Option<String>,
}

impl CronJob {
    pub fn try_schedule(&mut self) -> bool {
        if self.check_schedule() {
            self.run_id = Some(Alphanumeric.sample_string(&mut rand::thread_rng(), 8));
            self.channels = Some(crossbeam_channel::bounded(1));
            if self.start_time.is_none() {
                self.start_time = Some(Utc::now());
            }
            self.handler = Some(self.run());
            return true;
        }
        false
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
            },
            CronJobType::Global(job) | CronJobType::Function(job)  => {
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
            },
        }
    }
}
pub struct CronFrame {
    pub cron_jobs: Vec<CronJob>,
    logger: Handle,
}
impl CronFrame {
    pub fn init() -> Self {
        let logger = Self::default_logger();
        let mut frame = CronFrame {
            cron_jobs: vec![],
            logger,
        };

        info!("CronFrame Initialization Start.");
        info!("Colleting Global Jobs.");
        // get the automatically collected global jobs
        for job_builder in inventory::iter::<JobBuilder> {
            let cron_job = job_builder.build();
            info!("Found Global Job \"{}\"", cron_job.name);
            frame.cron_jobs.push(cron_job)
        }
        info!("Global Jobs Collected.");
        info!("CronFrame Initialization Complete.");
        frame
    }

    pub fn add_job(&mut self, job: CronJob) {
        self.cron_jobs.push(job)
    }

    pub fn scheduler(mut self) {
        let scheduler = move || loop {
            // sleep some otherwise the cpu consumption goes to the moon
            thread::sleep(Duration::milliseconds(500).to_std().unwrap());

            for cron_job in &mut self.cron_jobs {
                if cron_job.handler.is_none() {
                    if cron_job.check_timeout() {
                        info!("job @ {} - Reached Timeout", cron_job.name);
                        // TODO remove timedout job from list of actives?
                        continue;
                    }
                    let scheduled = cron_job.try_schedule();
                    if scheduled {
                        info!(
                            "job @ {} # {} - Scheduled.",
                            cron_job.name,
                            cron_job.run_id.as_ref().unwrap()
                        );
                    }
                } else {
                    let _tx = &cron_job.channels.as_ref().unwrap().0;
                    let rx = &cron_job.channels.as_ref().unwrap().1;

                    match rx.try_recv() {
                        Ok(message) => {
                            if message == "JOB_COMPLETE" {
                                info!(
                                    "job @ {} # {} - Completed.",
                                    cron_job.name,
                                    cron_job.run_id.as_ref().unwrap()
                                );
                                cron_job.handler = None;
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
        info!("Scheduler Thread Started.");
    }

    fn default_logger() -> Handle {
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

        let handle = log4rs::init_config(config).unwrap();

        handle
    }
}
