extern crate lazy_static;
use chrono::DateTime;
pub use chrono::{Duration, Utc};
pub use cron::Schedule;
pub use cronmacro::{cron, cron_impl, cron_obj, job};
use crossbeam_channel::{Receiver, Sender};
pub use lazy_static::lazy_static;
use rand::distributions::{Alphanumeric, DistString};
pub use std::any::Any;
pub use std::any::{self, TypeId};
pub use std::str::FromStr;
pub use std::thread;
pub use std::{collections::HashMap, sync::Mutex, thread::JoinHandle, vec};

// necessary to gather all the annotated jobs automatically
inventory::collect!(JobBuilder<'static>);
inventory::collect!(CronObj);

pub struct CronObj {
    pub helper: fn(arg: &dyn Any) -> JobBuilder,
}

impl CronObj {
    pub const fn new(helper: fn(arg: &dyn Any) -> JobBuilder) -> Self {
        CronObj { helper }
    }
}

pub enum JobBuilder<'a> {
    Function {
        job: fn(arg: &dyn Any),
        cron_expr: &'a str,
        timeout: &'a str,
    },
    Method {
        job: fn(arg: &dyn Any),
        cron_expr: String,
        timeout: &'a str,
    },
}

impl<'a> JobBuilder<'a> {
    pub const fn from_fn(job: fn(&dyn Any), cron_expr: &'a str, timeout: &'a str) -> Self {
        JobBuilder::Function {
            job,
            cron_expr,
            timeout,
        }
    }

    pub const fn from_met(job: fn(&dyn Any), cron_expr: String, timeout: &'a str) -> Self {
        JobBuilder::Method {
            job,
            cron_expr,
            timeout,
        }
    }

    pub fn build(&self) -> CronJob {
        match self {
            Self::Function {
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
                    job: job.clone(),
                    schedule,
                    timeout,
                    handler: None,
                    channels: None,
                    start_time: None,
                    run_id: None,
                }
            }
            Self::Method {
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
                    job: job.clone(),
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

/// # CronJob
///
/// Internal structure for the representation of a single cronjob.
///
/// The expansion of the cron macro annotation provides:
/// - the job function pointer (the original annotated function)
/// - the get info function pointer (Schedule and Timeout)
pub struct CronJob {
    job: fn(arg: &dyn Any),
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
        let job = self.job.clone();
        let tx = self.channels.as_ref().unwrap().0.clone();
        let _rx = self.channels.as_ref().unwrap().1.clone();
        let schedule = self.schedule.clone();
        let run_id = self.run_id.as_ref().unwrap().clone();

        let job_thread = move || loop {
            let now = Utc::now();
            if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                let until_next = next - now;
                thread::sleep(until_next.to_std().unwrap());
                print!("thread of job #{run_id} at time {}: ", Utc::now());
                job(&());
                let _ = tx.send("JOB_COMPLETE".to_string());
                break;
            }
        };

        thread::spawn(job_thread)
    }
}

/// # CronFrame
///
/// This is where the annotated functions are made into cronjobs.
///    Users::cron_helper_get_jobs(&user);
/// The `init()` method builds an instance collecting all the cronjobs.
///
/// The `schedule()` method provides the scheduling for the jobs and retrieves their thread handle.
///
pub struct CronFrame {
    pub cron_jobs: Vec<CronJob>,
}
impl CronFrame {
    pub fn init() -> Self {
        let mut frame = CronFrame { cron_jobs: vec![] };

        // get the automatically collected global jobs
        for job_builder in inventory::iter::<JobBuilder> {
            frame.cron_jobs.push(job_builder.build())
        }

        // // get the automatically collected object jobs
        // for cron_obj in inventory::iter::<CronObj> {
        //     let job_builder = (cron_obj.helper)(&());
        //     frame.cron_jobs.push(job_builder.build())
        // }

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
                        continue;
                    }
                    let scheduled = cron_job.try_schedule();
                    if scheduled {
                        println!("JOB #{} Scheduled.", cron_job.run_id.as_ref().unwrap());
                    }
                } else {
                    let _tx = &cron_job.channels.as_ref().unwrap().0;
                    let rx = &cron_job.channels.as_ref().unwrap().1;

                    match rx.try_recv() {
                        Ok(message) => {
                            if message == "JOB_COMPLETE" {
                                println!("JOB #{} Completed.", cron_job.run_id.as_ref().unwrap());
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
    }
}