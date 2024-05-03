pub use cronmacro::cron;
pub use cron::Schedule;
pub use std::str::FromStr;
pub use std::thread;
use std::thread::JoinHandle;
pub use chrono::{Utc, Duration};



/// # CronJob 
/// 
/// Internal structure for the representation of a single cronjob.
/// 
/// The expansion of the cron macro annotation provides:
/// - the job function pointer (the original annotated function)
/// - the get info function pointer (Schedule and Timeout)
/// 
pub struct CronJob{
    job: fn(),
    get_info: fn() -> (Schedule, i64),
}

impl CronJob{
    pub const fn new(job: fn(),  get_info: fn() -> (Schedule, i64)) -> Self {
        CronJob { job, get_info}
    }

    pub fn run(&self) -> JoinHandle<()> {
        let job = self.job.clone();
        let schedule = (self.get_info)().0;
        let timeout = (self.get_info)().1;

        let job_thread = move ||{ 
            loop {
                let now = Utc::now();
                if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                    let until_next = next - now;
                    thread::sleep(until_next.to_std().unwrap());
                    job();
                }
            }
        };
        thread::spawn(job_thread)
    }
}

// necessary to gather all the annotated jobs automatically
inventory::collect!(CronJob);

/// # CronFrame
/// 
/// This is where the annotated functions are made into cronjobs.
/// 
/// The `init()` method builds an instance collecting all the cronjobs.
/// 
/// The `schedule()` method provides the scheduling for the jobs and retrieves their thread handle.
/// 
pub struct CronFrame<'a>{
    cronjobs: Vec<&'a CronJob>,
    handlers: Vec<JoinHandle<()>>,
}

impl<'a> CronFrame<'a>{
    pub fn init() -> Self{
        let mut frame = CronFrame {
            cronjobs: vec![],
            handlers: vec![],
        };

        // get the automatically collected jobs  
        for job in inventory::iter::<CronJob> {
            frame.cronjobs.push(job)
        }

        frame
    }

    pub fn schedule(mut self) {
        for cronjob in &self.cronjobs{
            let handler = cronjob.run();
            self.handlers.push(handler);
        }

        loop{
            if self.handlers.len() == 0{
                break
            }
        }
    }
}