use std::{
    alloc::GlobalAlloc,
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use chrono::{Duration, Utc};
use crossbeam_channel::{Receiver, Sender};
use rocket::Shutdown;

use crate::{
    cronjob::CronJob, job_builder::JobBuilder, logger, web_server, CronFilter, CronJobType,
};

pub struct CronFrame {
    pub cron_jobs: Mutex<Vec<CronJob>>,
    handlers: Mutex<HashMap<String, JoinHandle<()>>>,
    logger: Option<log4rs::Handle>,
    pub web_server_channels: (Sender<Shutdown>, Receiver<Shutdown>),
    pub filter: Option<CronFilter>,
    server_handle: Mutex<Option<Shutdown>>,
    pub quit: Mutex<bool>,
}

impl CronFrame {
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
            quit: Mutex::new(false),
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

    pub fn scheduler<'a>(self: &Arc<Self>) -> Arc<Self> {
        let cronframe = self.clone();
        let ret = cronframe.clone();

        let scheduler = move || loop {
            // sleep some otherwise the cpu consumption goes to the moon
            std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());

            if *cronframe.quit.lock().unwrap() {
                break;
            }

            let mut cron_jobs = cronframe.cron_jobs.lock().unwrap();
            let mut jobs_to_remove = Vec::new();

            for (i, cron_job) in &mut (*cron_jobs).iter_mut().enumerate() {
                if let Some(filter) = &cronframe.filter {
                    let job_type = match cron_job.job {
                        CronJobType::Global(_) => CronFilter::Global,
                        CronJobType::Function(_) => CronFilter::Function,
                        CronJobType::Method(_) => CronFilter::Method,
                    };

                    if job_type != *filter {
                        continue;
                    }
                }
                
                // if the job dropped remove it, for method jobs
                let job_id = format!("{} ID#{}", cron_job.name, cron_job.id);

                let rx = cron_job
                    .status_channels
                    .clone()
                    .expect("error: unwrapping rx channel")
                    .1;

                match rx.try_recv() {
                    Ok(message) => {
                        if message == "JOB_DROP" {
                            info!(
                                "job @{} - Dropped",
                                job_id
                            );
                            jobs_to_remove.push(i);
                            continue;
                        }
                    }
                    Err(_error) => {}
                }

                // if the job_id key is not in the hashmap then attempt to schedule it
                // if scheduling is a success then add the key to the hashmap
                if !cronframe.handlers.lock().unwrap().contains_key(&job_id) {
                    // if the job timed-out than skip to the next job
                    if cron_job.check_timeout() {
                        // TODO make a timed-out job resume on the following day
                        if !cron_job.timeout_notified {
                            info!("job @{} - Reached Timeout", job_id);
                            cron_job.timeout_notified = true;
                        }
                        continue;
                    }

                    let handle = (*cron_job).try_schedule();

                    if handle.is_some() {
                        cronframe
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
                        .status_channels
                        .clone()
                        .expect("error: unwrapping tx channel")
                        .0;
                    let rx = cron_job
                        .status_channels
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
                                cronframe.handlers.lock().unwrap().remove(job_id.as_str());
                                //cron_job.status_channels = None;
                                cron_job.run_id = None;
                            } else if message == "JOB_ABORT" {
                                info!(
                                    "job @{} RUN_ID#{} - Aborted",
                                    job_id,
                                    cron_job.run_id.as_ref().unwrap()
                                );
                                cronframe.handlers.lock().unwrap().remove(job_id.as_str());
                                //cron_job.status_channels = None;
                                cron_job.run_id = None;
                                cron_job.failed = true;
                            }
                        }
                        Err(_error) => {}
                    }
                }
            }

            // cleanup of dropped method jobs
            if !jobs_to_remove.is_empty() {
                let num_jobs = jobs_to_remove.len();
                for i in 0..num_jobs {
                    cron_jobs.remove(jobs_to_remove[i]);
                    for j in i + 1..num_jobs {
                        jobs_to_remove[j] -= 1;
                    }
                }
            }
        };

        std::thread::spawn(scheduler);
        info!("CronFrame Scheduler Running");
        ret
    }

    pub fn quit(self: &Arc<Self>) {
        info!("CronFrame Scheduler Shutdown");

        let cronframe = self.clone();
        *cronframe.quit.lock().unwrap() = true;

        let handles = cronframe.handlers.lock().unwrap();

        for handle in handles.iter() {
            while !handle.1.is_finished() {
                // do some waiting until all job threads have terminated.
            }
        }

        // quit the web server
        cronframe
            .server_handle
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .notify();
    }
}
