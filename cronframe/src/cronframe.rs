use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use chrono::Duration;

use crate::{cronjob::CronJob, job_builder::JobBuilder, logger, web_server};

pub struct CronFrame {
    pub cron_jobs: Mutex<Vec<CronJob>>,
    handlers: Mutex<HashMap<String, JoinHandle<()>>>,
    _logger: log4rs::Handle,
}

impl CronFrame {
    pub fn init() -> Arc<CronFrame> {
        let _logger = logger::default_logger();
        let frame = CronFrame {
            cron_jobs: Mutex::new(vec![]),
            handlers: Mutex::new(HashMap::new()),
            _logger,
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
        let frame = Arc::new(frame);
        let ret_frame = frame.clone();

        std::thread::spawn(move || web_server::server(frame));
        info!("CronFrame Server Running");
        ret_frame
    }

    pub fn add_job(&mut self, job: CronJob) {
        self.cron_jobs.lock().unwrap().push(job)
    }

    pub fn scheduler(self: &Arc<Self>) {
        let instance = self.clone();

        let scheduler = move || loop {
            // sleep some otherwise the cpu consumption goes to the moon
            std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());

            let mut cron_jobs = instance.cron_jobs.lock().unwrap();

            for cron_job in &mut (*cron_jobs) {
                let job_id = format!("{} ID#{}", cron_job.name, cron_job.id);

                // if the job_id key is not in the hashmap then attempt to schedule it
                // if scheduling is a success then add the key to the hashmap
                if !instance.handlers.lock().unwrap().contains_key(&job_id) {
                    if cron_job.check_timeout() {
                        info!("job @{} - Reached Timeout", job_id);
                        // TODO remove timed-out job from list of actives?
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
                                cron_job.failed = false;
                            } else if message == "JOB_ABORT" {
                                info!(
                                    "job @{} RUN_ID#{} - Aborted",
                                    job_id,
                                    cron_job.run_id.as_ref().unwrap()
                                );
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
}
