use std::{
    any::Any,
    str::FromStr,
    sync::Arc,
    thread::{self, JoinHandle}
};

use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use crossbeam_channel::{Receiver, Sender};
use syn::TraitItemMacro;

use crate::{utils, CronJobType, ID_SIZE};

#[derive(Debug, Clone)]
pub struct CronJob {
    pub name: String,
    pub id: String,
    pub job: CronJobType,
    pub schedule: Schedule,
    pub timeout: Option<Duration>,
    pub channels: Option<(Sender<String>, Receiver<String>)>,
    pub start_time: Option<DateTime<Utc>>,
    pub run_id: Option<String>,
    pub instance: Option<Arc<Box<dyn Any + Send + Sync>>>,
    pub failed: bool,
}

impl CronJob {
    pub fn try_schedule(&mut self) -> Option<JoinHandle<()>> {
        if self.check_schedule() {
            self.run_id = Some(utils::generate_id(ID_SIZE));
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
        if self.failed {
            "Failed".to_string()
        } else if self.check_timeout() {
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
        let new_tx = tx.clone();

        let tick_thread = move || loop{
            std::thread::sleep(chrono::Duration::milliseconds(500).to_std().unwrap());
            let _ = new_tx.send("JOB_WORKING".to_string());
        };

        match self.job {
            CronJobType::Method(job) => {
                let instance = self.instance.clone().unwrap();
                let job_thread = move || {
                    std::thread::spawn(tick_thread);
                    loop {
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
                };
                std::thread::spawn(job_thread)
            }
            CronJobType::Global(job) | CronJobType::Function(job) => {
                let job_thread = move || {
                    std::thread::spawn(tick_thread);
                    loop {
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
                };
                std::thread::spawn(job_thread)
            }
        }
    }
}
