extern crate lazy_static;
pub use lazy_static::lazy_static;
pub use std::any::{self, TypeId};
use chrono::DateTime;
pub use chrono::{Duration, Utc};
pub use cron::Schedule;
pub use cronmacro::{cron, cron_obj, cron_impl, job};
use crossbeam_channel::{Receiver, Sender};
use rand::distributions::{Alphanumeric, DistString};
pub use std::str::FromStr;
pub use std::thread;
pub use std::{collections::HashMap, sync::Mutex, thread::JoinHandle, vec};

// necessary to gather all the annotated jobs automatically
inventory::collect!(JobBuilder<'static>);
inventory::collect!(CronObj<'static>);

pub struct CronObj<'a>{
    helper: fn() -> JobBuilder<'a>,
}

impl CronObj<'static>{
    pub const fn new(helper: fn() -> JobBuilder<'static>) -> Self{
        CronObj{helper}
    }
}

/// # CronJob
///
/// Internal structure for the representation of a single cronjob.
///
/// The expansion of the cron macro annotation provides:
/// - the job function pointer (the original annotated function)
/// - the get info function pointer (Schedule and Timeout)
///
///
///

pub struct JobBuilder<'a> {
    job: fn(),
    cron_expr: &'a str,
    timeout: &'a str,
}
impl<'a> JobBuilder<'a> {
    pub const fn new(job: fn(), cron_expr: &'a str, timeout: &'a str) -> Self {
        JobBuilder {
            job,
            cron_expr,
            timeout,
        }
    }

    pub fn build(&self) -> CronJob {
        let job = self.job;
        let schedule =
            Schedule::from_str(self.cron_expr).expect("Failed to parse cron expression!");
        let timeout: i64 = self.timeout.parse().expect("Failed to parse timeout!");

        let timeout = if timeout > 0 {
            Some(Duration::milliseconds(timeout))
        } else {
            None
        };

        CronJob {
            job,
            schedule,
            timeout,
            handler: None,
            channels: None,
            start_time: None,
            run_id: None,
        }
    }
}

pub struct CronJob {
    job: fn(),
    schedule: Schedule,
    timeout: Option<Duration>,
    handler: Option<JoinHandle<()>>,
    channels: Option<(Sender<String>, Receiver<String>)>,
    start_time: Option<DateTime<Utc>>,
    run_id: Option<String>,
}

impl CronJob{
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
                job();
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
///
/// The `init()` method builds an instance collecting all the cronjobs.
///
/// The `schedule()` method provides the scheduling for the jobs and retrieves their thread handle.
///
pub struct CronFrame {
    cron_jobs: Vec<CronJob>,
}
impl CronFrame {
    pub fn init() -> Self {
        let mut frame = CronFrame { cron_jobs: vec![] };

        // get the automatically collected global jobs
        for job_builder in inventory::iter::<JobBuilder> {
            frame.cron_jobs.push(job_builder.build())
        }

        // get the automatically collected object jobs
        for cron_obj in inventory::iter::<CronObj> {
            let job_builder = (cron_obj.helper)();
            frame.cron_jobs.push(job_builder.build())
        }

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