//! The Core Type of the Library

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use chrono::Duration;
use crossbeam_channel::{Receiver, Sender};
use rocket::Shutdown;

use crate::{
    config::read_config, cronjob::CronJob, job_builder::JobBuilder, logger, web_server, CronFilter,
    CronJobType,
};

const GRACE_DEFAULT: u32 = 250;

/// This is the type that provides the scheduling and management of jobs.
///
/// It needs to be initialised once to setup the web server and gather global jobs.
///
/// Either one of the `start_scheduler` or `run` method must be invoked for it to actually start.
/// ```
/// # #[macro_use] extern crate cronframe_macro;
/// # use cronframe::CronFrame;
/// fn main(){
///     let cronframe = CronFrame::default(); // this a shorthand for Cronframe::init(None, true);
///     cronframe.start_scheduler(); //does not keep main alive
///     //cronframe.keep_alive(); // keeps main thread alive
///     //cronframe.run(); //starts the scheduler, keeps main alive
/// }
pub struct CronFrame {
    pub cron_jobs: Mutex<Vec<CronJob>>,
    job_handles: Mutex<HashMap<String, JoinHandle<()>>>,
    _logger: Option<log4rs::Handle>,
    pub web_server_channels: (Sender<Shutdown>, Receiver<Shutdown>),
    pub filter: Option<CronFilter>,
    server_handle: Mutex<Option<Shutdown>>,
    pub quit: Mutex<bool>,
    pub grace: u32,
    pub running: Mutex<bool>,
}

impl CronFrame {
    /// It returns an `Arc<CronFrame>` which is used in the webserver and can be used to start the scheduler.
    /// ```
    /// # #[macro_use] extern crate cronframe_macro;
    /// # use cronframe::CronFrame;
    /// fn main(){
    ///     // inits the library and gathers global jobs if there are any
    ///     // does not start the scheduler, only the web server is live
    ///     let cronframe = CronFrame::default(); // this a shorthand for Cronframe::init(None, true);
    ///     //cronframe.keep_alive(); // keeps main thread alive
    ///     //cronframe.run(); //starts the scheduler, keeps main alive
    /// }
    /// ```
    pub fn default() -> Arc<CronFrame> {
        CronFrame::init(None, true)
    }

    /// Init function of the library, it takes two agruments:
    /// ```text
    /// filter: Option<CronFilter>
    /// use_logger: bool
    /// ```
    ///
    /// It manages:
    /// - the logger setup if use_logger is true
    /// - the creation of the CronFrame Instance
    /// - the collection of global jobs
    /// - the setup of the web server
    ///
    /// It returns an `Arc<CronFrame>` which is used in the webserver and to start the scheduler.
    pub fn init(filter: Option<CronFilter>, use_logger: bool) -> Arc<CronFrame> {
        println!("Starting Cronframe...");
        let logger = if use_logger {
            Some(logger::rolling_logger())
        } else {
            None
        };

        let frame = CronFrame {
            cron_jobs: Mutex::new(vec![]),
            job_handles: Mutex::new(HashMap::new()),
            _logger: logger,
            web_server_channels: crossbeam_channel::bounded(1),
            filter,
            server_handle: Mutex::new(None),
            quit: Mutex::new(false),
            grace: {
                if let Some(config_data) = read_config() {
                    if let Some(scheduler_data) = config_data.scheduler {
                        scheduler_data.grace.unwrap_or_else(|| 250)
                    } else {
                        GRACE_DEFAULT
                    }
                } else {
                    GRACE_DEFAULT
                }
            },
            running: Mutex::new(false),
        };

        info!("CronFrame Init Start");
        info!("Graceful Period {} ms", frame.grace);
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
        let frame = Arc::new(frame);
        let server_frame = frame.clone();

        std::thread::spawn(move || web_server::web_server(server_frame));

        *frame
            .server_handle
            .lock()
            .expect("web server handle unwrap error") = match frame.web_server_channels.1.recv() {
            Ok(handle) => Some(handle),
            Err(error) => {
                println!("Web server shutdown handle error: {error}");
                None
            }
        };

        info!("CronFrame Web Server Running");
        frame
    }

    /// It adds and existing job to the cronframe instance to the job pool
    /// Used in the cf_gather_mt and cf_gather_fn
    pub fn add_job(self: &Arc<CronFrame>, job: CronJob) -> Arc<CronFrame> {
        self.cron_jobs
            .lock()
            .expect("add_job unwrap error on lock")
            .push(job);
        self.clone()
    }

    // It crates a new job which will be classified as a global job and adds to the job pool
    pub fn new_job(
        self: Arc<CronFrame>,
        name: &str,
        job: fn(),
        cron_expr: &str,
        timeout: &str,
    ) -> Arc<CronFrame> {
        self.add_job(JobBuilder::global_job(name, job, cron_expr, timeout).build())
    }

    /// It spawns a thread which manages the scheduling of the jobs and termination of jobs.
    ///
    /// This method returns after spawning the scheduler.
    ///
    /// Keeping the main thread alive is left to the user.
    ///
    /// Use the `run` method to spawn the scheduler and keep main thread alive.
    /// ```
    /// # #[macro_use] extern crate cronframe_macro;
    /// # use cronframe::CronFrame;
    /// fn main(){
    ///     let cronframe = CronFrame::default().start_scheduler();
    /// }
    /// ```
    pub fn start_scheduler<'a>(self: &Arc<Self>) -> Arc<Self> {
        let cronframe = self.clone();
        let ret = cronframe.clone();

        // if already running, return
        if *self.running.lock().unwrap() {
            return ret;
        }

        *cronframe
            .running
            .lock()
            .expect("running unwrap error in quit start_scheduler method") = true;
        *cronframe
            .quit
            .lock()
            .expect("quit unwrap error in start_scheduler method") = false;

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
                        CronJobType::CLI => CronFilter::CLI,
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

                // check if the daily timeout expired and reset it if need be
                cron_job.reset_timeout();

                // if there is no handle for the job see if it need to be scheduled
                if !job_handlers.contains_key(&job_id) && !to_be_deleted {
                    if cron_job.suspended {
                        continue;
                    }

                    // if the job timed-out than skip to the next job
                    if cron_job.check_timeout() {
                        if !cron_job.timeout_notified {
                            info!("job @{} - Reached Timeout", job_id);
                            cron_job.timeout_notified = true;
                        }
                        continue;
                    }

                    let handle = (*cron_job).try_schedule(cronframe.grace);

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
    /// Use the `scheduler` method if you only need to start the scheduler.
    pub fn run(self: &Arc<Self>) {
        self.start_scheduler().keep_alive();
    }

    /// Stop the scheduler and wait for the jobs to finish
    pub fn stop_scheduler(self: &Arc<Self>) {
        if *self.running.lock().unwrap() {
            info!("CronFrame Scheduler Shutdown");
            *self.running.lock().unwrap() = false;

            let cronframe = self.clone();

            *cronframe
                .quit
                .lock()
                .expect("quit unwrap error in stop scheduler method") = true;

            let handles = cronframe
                .job_handles
                .lock()
                .expect("job handles unwrap error in stop scheduler method");

            for handle in handles.iter() {
                while !handle.1.is_finished() {
                    // do some waiting until all job threads have terminated.
                }
            }
        }
    }

    /// Function to call for a graceful shutdown of the library
    /// ```
    /// # #[macro_use] extern crate cronframe_macro;
    /// # use cronframe::CronFrame;
    ///
    /// fn main(){
    ///     let cronframe = CronFrame::default();
    ///     // do somthing...
    ///     cronframe.start_scheduler();
    ///     // do other things...
    ///     cronframe.quit();
    /// }
    /// ```
    pub fn quit(self: &Arc<Self>) {
        self.stop_scheduler();
        info!("CronFrame Shutdown");

        // quit the web server
        self.server_handle
            .lock()
            .expect("web server unwrap error in quit method")
            .clone()
            .expect("web server unwrap error after clone in quit method")
            .notify();
    }
}
