use std::{any::Any, borrow::Borrow, str::FromStr, sync::Arc, thread::JoinHandle};

use chrono::{DateTime, Duration, Local, Utc};
use cron::Schedule;
use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use crate::CronJobType;

#[derive(Debug, Clone)]
pub struct CronJob {
    pub name: String,
    pub id: Uuid,
    pub job: CronJobType,
    pub schedule: Schedule,
    pub timeout: Option<Duration>,
    pub channels: Option<(Sender<String>, Receiver<String>)>,
    pub start_time: Option<DateTime<Utc>>,
    pub run_id: Option<Uuid>,
    pub instance: Option<Arc<Box<dyn Any + Send + Sync>>>,
    pub failed: bool,
}

impl CronJob {
    pub fn try_schedule(&mut self) -> Option<JoinHandle<()>> {
        if self.check_schedule() {
            self.run_id = Some(Uuid::new_v4());
            self.channels = Some(crossbeam_channel::bounded(1));

            if self.start_time.is_none() {
                self.start_time = Some(Utc::now());
            }

            match self.run() {
                Ok(handle) => Some(handle),
                Err(_error) => None,
            }
        } else {
            None
        }
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
            .upcoming(Local)
            .into_iter()
            .next()
            .unwrap()
            .to_string()
    }

    pub fn get_run_id(&self) -> String {
        match &self.run_id {
            Some(uuid) => uuid.to_string(),
            None => "None".into(),
        }
    }

    pub fn run(&self) -> std::io::Result<JoinHandle<()>> {
        let cron_job = self.clone();
        let tx = self.channels.as_ref().unwrap().0.clone();
        let _rx = self.channels.as_ref().unwrap().1.clone();
        let schedule = self.schedule.clone();
        let job_id = format!("{} ID#{}", self.name, self.id);
        let run_id = self.run_id.as_ref().unwrap().clone();

        let job_thread = move || {
            let now = Utc::now();
            if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                let until_next = next - now;
                std::thread::sleep(until_next.to_std().unwrap());
                info!("job @{job_id} RUN_ID#{run_id} - Execution");
                match cron_job.job {
                    CronJobType::Global(job) | CronJobType::Function(job) => job(),
                    CronJobType::Method(job) => job(cron_job.instance.unwrap()),
                }
            }
        };

        let control_thread = move || {
            let job_handle = std::thread::spawn(job_thread); 

            while !job_handle.is_finished(){
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
