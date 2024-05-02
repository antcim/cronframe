pub use cronmacro::cron;
pub use cron::Schedule;
pub use std::str::FromStr;
pub use std::thread;
use std::thread::JoinHandle;
pub use chrono::Utc;

// this crate ensures that both marcos and types it dependes on can be exported for public use
// necessary since proc-macro crates can only export with pub elements that have a #[proc...] annotation

pub struct CronJob{
    job: fn(),
    get_schedule: fn() -> Schedule,
}

impl CronJob{
    pub const fn new(job: fn(), get_schedule: fn() -> Schedule) -> Self {
        CronJob { job, get_schedule}
    }

    pub fn run(&self) -> JoinHandle<()> {
        let job = self.job.clone();
        let schedule = (self.get_schedule)();

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

inventory::collect!(CronJob);

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