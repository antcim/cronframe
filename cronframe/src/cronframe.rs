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
    cronjob::{self, CronJob},
    job_builder::JobBuilder,
    logger, web_server, CronFilter, CronJobType,
};

pub struct CronFrame {
    pub cron_jobs: Mutex<Vec<CronJob>>,
    job_handles: Mutex<HashMap<String, JoinHandle<()>>>,
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
        let logger = if use_logger {
            Some(logger::rolling_logger())
        } else {
            None
        };

        let mut frame = CronFrame {
            cron_jobs: Mutex::new(vec![]),
            job_handles: Mutex::new(HashMap::new()),
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
            frame
                .cron_jobs
                .lock()
                .expect("global job gathering error during init")
                .push(cron_job)
        }

        info!("Global Jobs Collected");
        info!("CronFrame Init Complete");

        info!("CronFrame Server Init");
        let mut frame = Arc::new(frame);
        let server_frame = frame.clone();

        std::thread::spawn(move || web_server::web_server(server_frame));

        *frame
            .server_handle
            .lock()
            .expect("web server handle unwrap error") = match frame.web_server_channels.1.recv() {
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
        self.cron_jobs
            .lock()
            .expect("add jobs unwrap error")
            .push(job)
    }

    pub fn scheduler<'a>(self: &Arc<Self>) -> Arc<Self> {
        let cronframe = self.clone();
        let ret = cronframe.clone();

        let scheduler = move || loop {
            // sleep some otherwise the cpu consumption goes to the moon
            std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());

            if *cronframe
                .quit
                .lock()
                .expect("quit unwrap error in scheduler")
            {
                break;
            }

            let mut cron_jobs = cronframe
                .cron_jobs
                .lock()
                .expect("cron jobs unwrap error in scheduler");
            let mut jobs_to_remove: Vec<usize> = Vec::new();

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

                let job_id = format!("{} ID#{}", cron_job.name, cron_job.id);

                // if cron_obj instance related to the job is dropped delete the job
                let to_be_deleted = if let Some((_, life_rx)) = cron_job.life_channels.clone() {
                    match life_rx.try_recv() {
                        Ok(message) => {
                            if message == "JOB_DROP" {
                                println!("job @{} - Dropped", job_id);
                                info!("job @{} - Dropped", job_id);
                                jobs_to_remove.push(i);
                                true
                            } else {
                                false
                            }
                        }
                        Err(_error) => false,
                    }
                } else {
                    false
                };

                // if the job_id key is not in the hashmap then attempt to schedule it
                // if scheduling is a success then add the key to the hashmap

                let mut job_handlers = cronframe
                    .job_handles
                    .lock()
                    .expect("job handles unwrap error in scheduler");

                if !job_handlers.contains_key(&job_id) && !to_be_deleted {
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
                        job_handlers.insert(
                            job_id.clone(),
                            handle.expect("job handle unwrap error after try_schedule"),
                        );
                        info!(
                            "job @{} RUN_ID#{} - Scheduled",
                            job_id,
                            cron_job.run_id.as_ref().expect("run_id unwrap error")
                        );
                    }
                }
                // the job is in the hashmap and running
                // check to see if it sent a message that says it finished or aborted
                else if let Some((_, status_rx)) = cron_job.status_channels.clone() {
                    match status_rx.try_recv() {
                        Ok(message) => {
                            if message == "JOB_COMPLETE" {
                                info!(
                                    "job @{} RUN_ID#{} - Completed",
                                    job_id,
                                    cron_job.run_id.as_ref().unwrap()
                                );
                                job_handlers.remove(job_id.as_str());
                                cron_job.run_id = None;
                            } else if message == "JOB_ABORT" {
                                info!(
                                    "job @{} RUN_ID#{} - Aborted",
                                    job_id,
                                    cron_job.run_id.as_ref().unwrap()
                                );
                                job_handlers.remove(job_id.as_str());
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

        *cronframe
            .quit
            .lock()
            .expect("quit unwrap error in quit method") = true;

        let handles = cronframe
            .job_handles
            .lock()
            .expect("job handles unwrap error in quit method");

        for handle in handles.iter() {
            while !handle.1.is_finished() {
                // do some waiting until all job threads have terminated.
            }
        }

        // quit the web server
        cronframe
            .server_handle
            .lock()
            .expect("web server unwrap error in quit method")
            .clone()
            .expect("web server unwrap error after clone in quit method")
            .notify();
    }
}
