pub use cronmacro::cron;
pub use cron::Schedule;
pub use std::str::FromStr;
pub use std::thread;
use std::thread::JoinHandle;
pub use chrono::Utc;

// this crate ensures that both marcos and types it dependes on can be exported for public use
// necessary since proc-macro crates can only export with pub elements that have a #[proc...] annotation

pub struct CronJob{
    job: fn() -> JoinHandle<fn()>
}

impl CronJob{
    pub const fn new(job: fn() -> JoinHandle<fn()>) -> Self {
        CronJob { job }
    }

    pub fn run(&self) -> JoinHandle<fn()> {
        (self.job)()
    }
}

inventory::collect!(CronJob);

pub struct CronFrame<'a>{
    cronjobs: Vec<&'a CronJob>,
    handlers: Vec<JoinHandle<fn()>>,
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
        for cronjob in self.cronjobs{
            let handler = cronjob.run();
            self.handlers.push(handler);
        }

        loop{
            let mut count = 0;
            for handler in &self.handlers{
                if handler.is_finished(){
                    count +=1;
                }
            }
            if count == self.handlers.len(){
                break
            }
        }
    }
}