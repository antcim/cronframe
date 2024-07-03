use std::{
    alloc::GlobalAlloc,
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use chrono::{Duration, Utc};
use crossbeam_channel::{Receiver, Sender};
use rocket::Shutdown;

use crate::{cronjob::CronJob, job_builder::JobBuilder, logger, web_server, CronJobType};

#[derive(PartialEq)]
pub enum CronFilter {
    Global,
    Function,
    Method,
}

pub struct CronFrame {
    pub cron_jobs: Mutex<Vec<CronJob>>,
    handlers: Mutex<HashMap<String, JoinHandle<()>>>,
    logger: Option<log4rs::Handle>,
    pub web_server_channels: (Sender<Shutdown>, Receiver<Shutdown>),
    pub filter: Option<CronFilter>,
    server_handle: Mutex<Option<Shutdown>>,
}

impl CronFrame {
    const QUIT: std::sync::Mutex<bool> = Mutex::new(false);

    pub fn default() -> Arc<CronFrame> {
        CronFrame::init(None, true)
    }

    pub fn init(filter: Option<CronFilter>, use_logger: bool) -> Arc<CronFrame> {
        let mut logger = None;

        if use_logger {
            logger = Some(logger::rolling_logger());
        }

        let mut frame = CronFrame {
            cron_jobs: Mutex::new(vec![]),
            handlers: Mutex::new(HashMap::new()),
            logger,
            web_server_channels: crossbeam_channel::bounded(1),
            filter,
            server_handle: Mutex::new(None),
        };

        info!("CronFrame Init Start");
        info!("Colleting Global Jobs");

        for job_builder in inventory::iter::<JobBuilder> {
            let cron_job = job_builder.clone().build();
            info!("Found Global Job \"{}\"", cron_job.name);
            frame.cron_jobs.lock().unwrap().push(cron_job)
        }

        info!("Global Jobs Collected");
        info!("CronFrame Init Complete");

        info!("CronFrame Server Init");
        let mut frame = Arc::new(frame);
        let server_frame = frame.clone();

        std::thread::spawn(move || web_server::web_server(server_frame));

        *frame.server_handle.lock().unwrap() = match frame.web_server_channels.1.recv() {
            Ok(handle) => {
                println!("Handle Received");
                Some(handle)
            }
            Err(error) => {
                println!("Handle ERROR: {error}");
                None
            }
        };

        info!("CronFrame Server Running");
        frame
    }

    pub fn add_job(&mut self, job: CronJob) {
        self.cron_jobs.lock().unwrap().push(job)
    }

    pub fn scheduler(self: &Arc<Self>) {
        let instance = self.clone();

        let scheduler = move || loop {
            // sleep some otherwise the cpu consumption goes to the moon
            std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());

            if *CronFrame::QUIT.lock().unwrap() {
                break;
            }

            let mut cron_jobs = instance.cron_jobs.lock().unwrap();

            for cron_job in &mut (*cron_jobs) {
                if let Some(filter) = &instance.filter {
                    let job_type = match cron_job.job {
                        CronJobType::Global(_) => CronFilter::Global,
                        CronJobType::Function(_) => CronFilter::Function,
                        CronJobType::Method(_) => CronFilter::Method,
                    };

                    if job_type != *filter {
                        continue;
                    }
                }

                let job_id = format!("{} ID#{}", cron_job.name, cron_job.id);

                // if the job_id key is not in the hashmap then attempt to schedule it
                // if scheduling is a success then add the key to the hashmap
                if !instance.handlers.lock().unwrap().contains_key(&job_id) {
                    // if the job timed-out than skip to the next job
                    if cron_job.check_timeout() {
                        // TODO make a timedout job resume on the following day
                        if !cron_job.timeout_notified {
                            info!("job @{} - Reached Timeout", job_id);
                            cron_job.timeout_notified = true;
                        }
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
                            "job @{} RUN_ID#{} - Scheduled",
                            job_id,
                            cron_job.run_id.as_ref().unwrap()
                        );
                    }
                }
                // the job is in the hashmap and running
                // check to see if it sent a message that says it finished or aborted
                else {
                    let tx = cron_job
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
                                    "job @{} RUN_ID#{} - Completed",
                                    job_id,
                                    cron_job.run_id.as_ref().unwrap()
                                );
                                instance.handlers.lock().unwrap().remove(job_id.as_str());
                                cron_job.channels = None;
                                cron_job.run_id = None;
                            } else if message == "JOB_ABORT" {
                                info!(
                                    "job @{} RUN_ID#{} - Aborted",
                                    job_id,
                                    cron_job.run_id.as_ref().unwrap()
                                );
                                instance.handlers.lock().unwrap().remove(job_id.as_str());
                                cron_job.channels = None;
                                cron_job.run_id = None;
                                cron_job.failed = true;
                            }
                        }
                        Err(_error) => {}
                    }
                    cron_job.check_timeout();
                }
            }
        };
        std::thread::spawn(scheduler);
        info!("CronFrame Scheduler Running");
    }

    pub fn quit(self: &Arc<Self>) {
        *CronFrame::QUIT.lock().unwrap() = true;

        let instance = self.clone();
        let logger = instance.logger.as_ref().unwrap().set_config(logger::trash_config());

        let tmp = instance
            .server_handle
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .notify();

        std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());
    }

    pub fn set_logger_config(self: &Arc<Self>){
        let instance = self.clone();
        let logger = instance.logger.as_ref().unwrap().set_config(logger::appender_config());
    }
}
