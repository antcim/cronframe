//! CronJob type, built by JobBuilder

use crate::{utils, CronJobType};
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use crossbeam_channel::{Receiver, Sender};
use std::{any::Any, process::Command, str::FromStr, sync::Arc, thread::JoinHandle};
use uuid::Uuid;

/// This type collects all necessary data for a cron job to be used in the scheduler.
///
/// While it could be used directly there are macros that build jobs for you.
#[derive(Debug, Clone)]
pub struct CronJob {
    pub suspended: bool,
    pub name: String,
    pub id: Uuid,
    pub job: CronJobType,
    pub schedule: Schedule,
    pub timeout: Option<Duration>,
    pub timeout_notified: bool,
    pub status_channels: Option<(Sender<String>, Receiver<String>)>,
    pub life_channels: Option<(Sender<String>, Receiver<String>)>,
    pub start_time: Option<DateTime<Utc>>,
    pub run_id: Option<Uuid>,
    pub method_instance: Option<Arc<Box<dyn Any + Send + Sync>>>,
    pub failed: bool,
}

impl CronJob {
    // this function is used in the scheduler thread to get a handle if the job has to be scheduled
    pub fn try_schedule(&mut self, grace_period: u32) -> Option<JoinHandle<()>> {
        if self.check_schedule() {
            self.run_id = Some(Uuid::new_v4());
            //self.status_channels = Some(crossbeam_channel::bounded(1));

            if self.start_time.is_none() {
                self.start_time = Some(Utc::now());
            }

            // we try to schedule the job and return its handle
            // in case scheduling fails for any conflict
            // we try again for as long as we are in the gracefull period

            if let Ok(handle) = self.run() {
                return Some(handle);
            }

            let gracefull_period = grace_period as i64;
            let first_try = Utc::now();
            let limit_time = first_try + Duration::milliseconds(gracefull_period);
            let mut graceful_log = false;

            while Utc::now() < limit_time {
                match self.run_graceful() {
                    Ok(handle) => {
                        if graceful_log {
                            let job_id = format!("{} ID#{}", self.name, self.id);
                            let run_id = self
                                .run_id
                                .expect("run_id unwrap error in try_schedule")
                                .to_string();
                            info!("job @{job_id} RUN_ID#{run_id} - Scheduled in Graceful Period");
                        }
                        return Some(handle);
                    }
                    Err(_error) => graceful_log = true,
                }
            }
        }
        None
    }

    // checks if a job's upcoming schedule is within the next second
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

    // the expected value is in milliseconds
    pub fn set_timeout(&mut self, value: i64) {
        self.timeout = if value > 0 {
            Some(Duration::milliseconds(value))
        } else {
            None
        };
    }

    // used to retrive a job's status to display it in the web server
    pub fn status(&self) -> String {
        if self.suspended {
            "Suspended".to_string()
        } else if self.check_timeout() {
            "Timed-Out".to_string()
        } else if self.run_id.is_some() {
            "Running".to_string()
        } else {
            "Awaiting Schedule".to_string()
        }
    }

    // method used by the web server the change the cron expression of a job
    pub fn set_schedule(&mut self, expression: &str) -> bool {
        let expr = expression.replace("slh", "/").replace("%20", " ");
        if let Ok(schedule) = Schedule::from_str(expr.as_str()) {
            self.schedule = schedule;
            return true;
        }
        false
    }

    // returns true if timeout expired
    pub fn check_timeout(&self) -> bool {
        if let Some(timeout) = self.timeout {
            if self.start_time.is_some() {
                let timeout = self
                    .start_time
                    .expect("start time unwrap error in check_timeout")
                    + timeout;

                if Utc::now() >= timeout {
                    return true;
                }
            }
        }
        false
    }

    // it resets the timeout if 24h have passed
    pub fn reset_timeout(&mut self) {
        if let Some(timeout) = self.timeout {
            if self.start_time.is_some() {
                let timeout = self
                    .start_time
                    .expect("start time unwrap error in timeout_reset")
                    + timeout;

                if Utc::now() >= timeout + Duration::hours(24) {
                    self.start_time = None;
                }
            }
        }
    }

    // get the schedule constructed from the cron expression
    pub fn schedule(&self) -> String {
        self.schedule.to_string()
    }

    // it returns the timeout or "None" if a timeout is not set
    pub fn timeout_to_string(&self) -> String {
        if self.timeout.is_some() {
            let timeout = self
                .timeout
                .expect("timeout unwrap error in timeout_to_string");
            format!(
                "{} s \n {} ms",
                timeout.num_seconds(),
                timeout.num_milliseconds()
            )
        } else {
            "None".into()
        }
    }

    // spells out the type of the job
    pub fn type_to_string(&self) -> String {
        match self.job {
            CronJobType::Global(_) => "Global".to_string(),
            CronJobType::Function(_) => "Function".to_string(),
            CronJobType::Method(_) => "Method".to_string(),
            CronJobType::CLI => "CLI".to_string(),
        }
    }

    // if the job is active it returns the schedule otherwise a message telling why there is no next schedule
    pub fn upcoming_utc(&self) -> String {
        if self.suspended {
            return "None due to scheduling suspension.".to_string();
        } else if self.check_timeout() {
            return "None due to timeout.".to_string();
        }
        self.schedule
            .upcoming(Utc)
            .into_iter()
            .next()
            .expect("schedule unwrap error in upcoming_utc")
            .to_string()
    }

    // if the job is active it returns the schedule otherwise a message telling why there is no next schedule
    pub fn upcoming_local(&self) -> String {
        if self.suspended {
            return "None due to scheduling suspension.".to_string();
        } else if self.check_timeout() {
            return "None due to timeout.".to_string();
        }
        utils::local_time(
            self.schedule
                .upcoming(Utc)
                .into_iter()
                .next()
                .expect("schedule unwrap error in upcoming_utc"),
        )
        .to_string()
    }

    // it returns the id of the current execution of the job, or "None" if it is not running
    pub fn run_id(&self) -> String {
        match &self.run_id {
            Some(uuid) => uuid.to_string(),
            None => "None".into(),
        }
    }

    // this spawns a control thread for the job that spawns a thread with the actual job
    pub fn run(&self) -> std::io::Result<JoinHandle<()>> {
        let cron_job = self.clone();
        let tx = self
            .status_channels
            .as_ref()
            .expect("tx unwap error in job run method")
            .0
            .clone();
        let _rx = self
            .status_channels
            .as_ref()
            .expect("rx unwap error in job run method")
            .1
            .clone();
        let schedule = self.schedule.clone();
        let job_id = format!("{} ID#{}", self.name, self.id);
        let run_id = self
            .run_id
            .as_ref()
            .expect("run_id unwap error in job run method")
            .clone();

        // the actual job thread
        // this is spawned form the control thread
        // it gets the next schedule, waits up to it and runs the job
        let job_thread = move || {
            let now = Utc::now();
            if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                let until_next = next - now;
                std::thread::sleep(until_next.to_std().unwrap());
                info!("job @{job_id} RUN_ID#{run_id} - Execution");
                match cron_job.job {
                    CronJobType::Global(job) | CronJobType::Function(job) => job(),
                    CronJobType::Method(job) => job(cron_job
                        .method_instance
                        .expect("method instance unwrap error in job thread")),
                    CronJobType::CLI => {
                        let home_dir = {
                            let tmp = home::home_dir().unwrap();
                            tmp.to_str().unwrap().to_owned()
                        };
                        let _build = Command::new(format!("./{}", cron_job.name))
                            .current_dir(format!("{home_dir}/.cronframe/cli_jobs"))
                            .status()
                            .expect("process failed to execute");
                    }
                }
            }
        };

        // the control thread handle is what gets returned to the cronframe
        // this allows to check for job completion or fail
        let control_thread = move || {
            let job_handle = std::thread::spawn(job_thread);

            while !job_handle.is_finished() {
                std::thread::sleep(Duration::milliseconds(250).to_std().unwrap());
            }

            match job_handle.join() {
                Ok(_) => {
                    let _ = tx.send("JOB_COMPLETE".to_string());
                }
                Err(_) => {
                    let _ = tx.send("JOB_ABORT".to_string());
                }
            };
        };

        std::thread::Builder::spawn(std::thread::Builder::new(), control_thread)
    }

    // same as run but it accounts for graceful period
    pub fn run_graceful(&self) -> std::io::Result<JoinHandle<()>> {
        let cron_job = self.clone();
        let tx = self
            .status_channels
            .as_ref()
            .expect("tx unwap error in job run_graceful method")
            .0
            .clone();
        let _rx = self
            .status_channels
            .as_ref()
            .expect("rx unwap error in job run_graceful method")
            .1
            .clone();
        let schedule = self.schedule.clone();
        let job_id = format!("{} ID#{}", self.name, self.id);
        let run_id = self.run_id.as_ref().unwrap().clone();

        // the actual job thread in graceful period
        // this is spawned form the control thread
        // it runs right away
        let job_thread = move || {
            let now = Utc::now();
            if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                let until_next = next - now;

                // we sleep only if we haven't yet reached the scheduled time
                // otherwise we immediatly execute the job
                if now < next {
                    std::thread::sleep(until_next.to_std().unwrap());
                }

                info!("job @{job_id} RUN_ID#{run_id} - Execution");
                match cron_job.job {
                    CronJobType::Global(job) | CronJobType::Function(job) => job(),
                    CronJobType::Method(job) => job(cron_job
                        .method_instance
                        .expect("method instance unwrap error in job thread")),
                    CronJobType::CLI => {
                        let home_dir = {
                            let tmp = home::home_dir().unwrap();
                            tmp.to_str().unwrap().to_owned()
                        };
                        let _build = Command::new(format!("./{}", cron_job.name))
                            .current_dir(format!("{home_dir}/.cronframe/cli_jobs"))
                            .status()
                            .expect("process failed to execute");
                    }
                }
            }
        };

        // the control thread handle is what gets returned to the cronframe
        // this allows to check for job completion or fail
        let control_thread = move || {
            let job_handle = std::thread::spawn(job_thread);

            while !job_handle.is_finished() {
                std::thread::sleep(Duration::milliseconds(250).to_std().unwrap());
            }

            match job_handle.join() {
                Ok(_) => {
                    let _ = tx.send("JOB_COMPLETE".to_string());
                }
                Err(_) => {
                    let _ = tx.send("JOB_ABORT".to_string());
                }
            };
        };

        std::thread::Builder::spawn(std::thread::Builder::new(), control_thread)
    }
}
