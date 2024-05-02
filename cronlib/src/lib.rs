pub use cronmacro::cron;
pub use cron::Schedule;
pub use std::str::FromStr;
pub use std::thread;
pub use chrono::Utc;

// this crate ensures that both marcos and types it dependes on can be exported for public use
// necessary since proc-macro crates can only export with pub elements that have a #[proc...] annotation

pub struct CronJob{
    job: fn()
}

impl CronJob{
    pub const fn new(job: fn()) -> Self {
        CronJob { job }
    }

    pub fn run(&self) {
        (self.job)()
    }
}

inventory::collect!(CronJob);

pub struct CronFrame<'a>{
    cronjobs: Vec<&'a CronJob>
}

impl<'a> CronFrame<'a>{
    pub fn init() -> Self{
        let mut frame = CronFrame {
            cronjobs: vec![]
        };

        for job in inventory::iter::<CronJob> {
            frame.cronjobs.push(job)
        }

        frame
    }

    pub fn schedule(self) {
        for cronjob in self.cronjobs{
            cronjob.run()
        }
    }
}