use crate::{
    config::{read_config, ConfigData},
    cronjob::{CronFilter, CronJob},
    job_builder::JobBuilder,
    logger, web_server,
};
use chrono::Duration;
use crossbeam_channel::{Receiver, Sender};
use rocket::Shutdown;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use uuid::Uuid;

#[derive(Debug)]
pub enum CFError {
    ServerStartup,
    TemplateDirectory,
    ServerShutdownHandle,
}

#[derive(Debug)]
pub enum SchedulerMessage {
    JobComplete,
    JobDrop,
    JobAbort,
}

pub struct CronFrame {
    job_pool: Mutex<HashMap<Uuid, CronJob>>,
    job_handles: Mutex<HashMap<Uuid, JoinHandle<()>>>,
    _logger: Option<log4rs::Handle>,
    rocket_channels: (Sender<Shutdown>, Receiver<Shutdown>),
    server_handle: Mutex<Option<Shutdown>>,
    pub quit: Mutex<bool>,
    pub running: Mutex<bool>,
    config: ConfigData,
}

impl CronFrame {
    pub fn init() -> Result<Arc<CronFrame>, CFError> {
        Self::with_config(read_config())
    }

    pub fn jobs(&self) -> &Mutex<HashMap<Uuid, CronJob>> {
        &self.job_pool
    }

    pub fn rocket_channels(&self) -> (Sender<Shutdown>, Receiver<Shutdown>) {
        self.rocket_channels.clone()
    }

    pub fn with_config(config: ConfigData) -> Result<Arc<CronFrame>, CFError> {
        println!("Starting CronFrame...");

        let logger = if config.logger.enabled {
            Some(logger::rolling_logger())
        } else {
            None
        };

        let frame = CronFrame {
            job_pool: Mutex::new(HashMap::new()),
            job_handles: Mutex::new(HashMap::new()),
            _logger: logger,
            rocket_channels: crossbeam_channel::bounded(1),
            server_handle: Mutex::new(None),
            quit: Mutex::new(false),
            running: Mutex::new(false),
            config,
        };

        info!("CronFrame Init Start");
        info!("Graceful Period {} ms", frame.config.scheduler.grace);
        info!("Colleting Global Jobs");

        for job_builder in inventory::iter::<JobBuilder> {
            let cron_job = job_builder.clone().build();
            info!("Found Global Job \"{}\"", cron_job.name);
            frame
                .job_pool
                .lock()
                .expect("global job gathering error during init")
                .insert(cron_job.id, cron_job);
        }

        info!("Global Jobs Collected");
        info!("CronFrame Init Complete");
        info!("CronFrame Server Init");

        let frame = Arc::new(frame);
        let server_frame = frame.clone();
        let running = Mutex::new(false);

        std::thread::spawn(move || web_server::web_server(server_frame));

        *frame
            .server_handle
            .lock()
            .expect("web server handle unwrap error") = match frame.rocket_channels.1.recv() {
            Ok(handle) => {
                *running.lock().unwrap() = true;
                Some(handle)
            }
            Err(error) => {
                error!("Web server shutdown handle error: {error}");
                None
            }
        };

        if *running.lock().unwrap() {
            info!(
                "CronFrame Web Server running at http://{}:{}",
                frame.config.webserver.ip, frame.config.webserver.port
            );
            println!(
                "CronFrame running at http://{}:{}",
                frame.config.webserver.ip, frame.config.webserver.port
            );
        } else {
            println!("Err(CronFrameError::ServerShutdownHandle)");
            return Err(CFError::ServerShutdownHandle);
        }

        Ok(frame)
    }

    /// It adds a CronJob instance to the job pool
    /// Used in the cf_gather_mt and cf_gather_fn
    pub fn add_job(self: &Arc<CronFrame>, job: CronJob) -> Arc<CronFrame> {
        self.job_pool
            .lock()
            .expect("add_job unwrap error on lock")
            .insert(job.id, job);
        self.clone()
    }

    pub fn job_filter(self: &Arc<CronFrame>) -> CronFilter {
        self.config.scheduler.job_filter
    }

    // It crates a new job classified as a global job and adds it to the job pool
    pub fn new_job(
        self: Arc<CronFrame>,
        name: &str,
        job: fn(),
        cron_expr: &str,
        timeout: &str,
    ) -> Arc<CronFrame> {
        self.add_job(JobBuilder::global_job(name, job, cron_expr, timeout).build())
    }

    pub fn start_scheduler<'a>(self: &Arc<Self>) -> Arc<Self> {
        let cronframe = self.clone();

        // if already running, return
        if *self.running.lock().unwrap() {
            return cronframe;
        }

        let cronframe_return = cronframe.clone();

        *cronframe
            .running
            .lock()
            .expect("running unwrap error in quit start_scheduler method") = true;
        *cronframe
            .quit
            .lock()
            .expect("quit unwrap error in start_scheduler method") = false;

        // closure containg the actual scheduler code
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

            if !*cronframe
                .running
                .lock()
                .expect("running unwrap error in scheduler")
            {
                break;
            }

            let mut cron_jobs = cronframe
                .job_pool
                .lock()
                .expect("cron jobs unwrap error in scheduler");

            let mut jobs_to_drop: Vec<Uuid> = Vec::new();

            for (job_id, cron_job) in &mut (*cron_jobs).iter_mut() {
                let filter = cronframe.config.scheduler.job_filter;

                // handle the job only if filter and job type match
                // No filter -> all job types are to be executed
                if filter != CronFilter::None {
                    if cron_job.job.type_to_filter() != filter {
                        continue;
                    }
                }

                // if cron_obj instance related to the job is dropped delete the job
                let to_be_dropped = if let Some((_, life_rx)) = cron_job.life_channels.clone() {
                    match life_rx.try_recv() {
                        Ok(message) => match message {
                            SchedulerMessage::JobDrop => {
                                info!("job name@{} - uuid#{} - Dropped", cron_job.name, job_id);
                                jobs_to_drop.push(*job_id);
                                true
                            }
                            _ => unreachable!(),
                        },
                        Err(_error) => false,
                    }
                } else {
                    false
                };

                // if the job_id key is not in the hashmap then attempt to schedule it
                // if scheduling is a success then add the key and handle to the hashmap

                let mut job_handles = cronframe
                    .job_handles
                    .lock()
                    .expect("job handles unwrap error in scheduler");

                // check if the daily timeout expired and reset it if need be
                cron_job.reset_timeout();

                // if there is no handle for the job see if it needs to be scheduled
                if !job_handles.contains_key(&job_id) && !to_be_dropped {
                    if cron_job.suspended {
                        continue;
                    }

                    // if the job timed-out than skip to the next job
                    if cron_job.check_timeout() {
                        if !cron_job.timeout_notified {
                            info!(
                                "job name@{} - uuid#{} - Reached Timeout",
                                cron_job.name, job_id
                            );
                            cron_job.timeout_notified = true;
                        }
                        continue;
                    }

                    let handle = (*cron_job).try_schedule(cronframe.config.scheduler.grace);

                    if handle.is_some() {
                        job_handles.insert(
                            job_id.clone(),
                            handle.expect("job handle unwrap error after try_schedule"),
                        );
                        info!(
                            "job name@{} - uuid#{} - run_uuid#{} - Scheduled",
                            cron_job.name,
                            job_id,
                            cron_job.run_id.as_ref().expect("run_uuid unwrap fail")
                        );
                    }
                }
                // the job is in the hashmap and running
                // check to see if it sent a message that says it finished or aborted
                else if let Some((_, status_rx)) = cron_job.status_channels.clone() {
                    match status_rx.try_recv() {
                        Ok(message) => match message {
                            SchedulerMessage::JobComplete => {
                                info!(
                                    "job name@{} - uuid#{} - run_uuid#{} - Completed",
                                    cron_job.name,
                                    job_id,
                                    cron_job.run_id.as_ref().expect("run_uuid unwrap fail")
                                );
                                job_handles.remove(job_id);
                                cron_job.run_id = None;
                            }
                            SchedulerMessage::JobAbort => {
                                info!(
                                    "job name@{} - uuid#{} - run_uuid#{} - Aborted",
                                    cron_job.name,
                                    job_id,
                                    cron_job.run_id.as_ref().expect("run_uuid unwrap fail")
                                );
                                job_handles.remove(job_id);
                                cron_job.run_id = None;
                                cron_job.failed = true;
                            }
                            _ => unreachable!(),
                        },
                        Err(_error) => {}
                    }
                }
            }

            let mut pool = cron_jobs;
            // drop function or methods jobs here
            for job_id in jobs_to_drop {
                pool.remove(&job_id);
            }
        };

        std::thread::spawn(scheduler);
        info!("CronFrame Scheduler Running");
        cronframe_return
    }

    /// This function can be used to keep the main thread alive after the scheduler has been started
    pub fn keep_alive(self: &Arc<Self>) {
        loop {
            std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());
            if *self.quit.lock().unwrap() {
                break;
            }
        }
    }

    /// Blocking method that starts the scheduler and keeps the main thread alive
    /// Use the `start_scheduler` method if need to start the scheduler and
    /// retain control of execution in main
    pub fn run(self: &Arc<Self>) {
        self.start_scheduler().keep_alive();
    }

    /// It quits the running scheduler instance
    pub fn stop_scheduler(self: &Arc<Self>) {
        info!("CronFrame Scheduler Shutdown");
        *self.running.lock().unwrap() = false;
    }

    pub fn quit(self: &Arc<Self>) {
        self.stop_scheduler();
        info!("CronFrame Shutdown");

        // wait for job handlers to finish
        let cronframe = self.clone();

        let handles = cronframe
            .job_handles
            .lock()
            .expect("job handles unwrap error in stop scheduler method");

        for handle in handles.iter() {
            while !handle.1.is_finished() {
                // do some waiting until all job threads have terminated
            }
        }

        // quit the web server
        self.server_handle
            .lock()
            .expect("web server unwrap error in quit method")
            .clone()
            .expect("web server unwrap error after clone in quit method")
            .notify();

        *self
            .quit
            .lock()
            .expect("quit unwrap error in stop scheduler method") = true;
    }
}
