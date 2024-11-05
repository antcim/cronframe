use crate::{cronframe::SchedulerMessage, utils};
use chrono::{DateTime, Duration, Local, Utc};
use cron::Schedule;
use crossbeam_channel::{Receiver, Sender};
use rocket::serde::Deserialize;
use std::{any::Any, process::Command, str::FromStr, sync::Arc, thread::JoinHandle};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CronJob {
    pub id: Uuid,
    pub name: String,
    pub job: CronJobType,
    pub suspended: bool,
    pub schedule: Schedule,
    pub timeout: Option<Duration>,
    pub timeout_notified: bool,
    pub status_channels: Option<(Sender<SchedulerMessage>, Receiver<SchedulerMessage>)>,
    pub life_channels: Option<(Sender<SchedulerMessage>, Receiver<SchedulerMessage>)>,
    pub start_time: Option<DateTime<Utc>>,
    pub run_id: Option<Uuid>,
    pub failed: bool,
}

#[derive(Debug, Clone)]
pub enum CronJobType {
    Global {
        job: fn(),
    },
    Function {
        job: fn(),
    },
    Method {
        instance: Arc<Box<dyn Any + Send + Sync>>,
        job: fn(Arc<Box<dyn Any + Send + Sync>>),
    },
    CLI {
        job_name: String,
    },
}

#[derive(Debug, PartialEq, Clone, Copy, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum CronFilter {
    None,
    Global,
    Function,
    Method,
    CLI,
}

impl CronJobType {
    pub fn run_job(&self) {
        match self {
            Self::Global { job } | Self::Function { job } => (job)(),
            Self::Method { instance, job } => (job)(instance.clone()),
            Self::CLI { job_name } => {
                let home_dir = {
                    let tmp = home::home_dir().unwrap();
                    tmp.to_str().unwrap().to_owned()
                };
                let _build = Command::new(format!("./{}", job_name))
                    .current_dir(format!("{home_dir}/.cronframe/cli_jobs"))
                    .status()
                    .expect("process failed to execute");
            }
        }
    }

    pub fn job_type(&self) -> String {
        match self {
            Self::Global { .. } => "Global".to_string(),
            Self::Function { .. } => "Function".to_string(),
            Self::Method { .. } => "Method".to_string(),
            Self::CLI { .. } => "CLI".to_string(),
        }
    }

    pub fn type_to_filter(&self) -> CronFilter {
        match self {
            CronJobType::Global { .. } => CronFilter::Global,
            CronJobType::Function { .. } => CronFilter::Function,
            CronJobType::Method { .. } => CronFilter::Method,
            CronJobType::CLI { .. } => CronFilter::CLI,
        }
    }
}

impl CronJob {
    pub fn try_schedule(&mut self, _grace_period: u32) -> Option<JoinHandle<()>> {
        if self.check_schedule() {
            self.run_id = Some(Uuid::new_v4());

            if self.start_time.is_none() {
                self.start_time = Some(Utc::now());
            }

            if let Ok(handle) = self.run() {
                return Some(handle);
            }

            // TODO add graceful period running logic
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
        self.job.job_type()
    }

    // if the job is active it returns the schedule otherwise a message telling why there is no next schedule
    pub fn upcoming_utc(&self) -> Option<DateTime<Utc>> {
        self.schedule.upcoming(Utc).into_iter().next()
    }

    // if the job is active it returns the schedule otherwise a message telling why there is no next schedule
    pub fn upcoming_local(&self) -> Option<DateTime<Local>> {
        if let Some(time) = self.schedule.upcoming(Utc).into_iter().next() {
            Some(utils::utc_to_local_time(time))
        } else {
            None
        }
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

        let run_id = cron_job
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

                info!(
                    "job name@{} - uuid#{} - run_uuid#{} - Execution",
                    cron_job.name, cron_job.id, run_id
                );
                cron_job.job.run_job();
            }
        };

        // the control thread handle is what gets returned to cronframe
        // this allows to check for job completion or fail
        let control_thread = move || {
            let job_handle = std::thread::spawn(job_thread);

            while !job_handle.is_finished() {
                std::thread::sleep(Duration::milliseconds(250).to_std().unwrap());
            }

            match job_handle.join() {
                Ok(_) => {
                    let _ = tx.send(SchedulerMessage::JobComplete);
                }
                Err(_) => {
                    let _ = tx.send(SchedulerMessage::JobAbort);
                }
            };
        };

        std::thread::Builder::spawn(std::thread::Builder::new(), control_thread)
    }
}
