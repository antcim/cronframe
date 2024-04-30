pub use cronmacro::cron;
pub use cron::Schedule;
pub use std::str::FromStr;
pub use std::thread;
pub use chrono::Utc;

// this crate ensures that both marco and  types it dependes on can be exported to public use
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
        CronFrame {
            cronjobs: vec![]
        }
    }

    pub fn schedule(mut self) -> Self {
        for job in inventory::iter::<CronJob> {
            self.cronjobs.push(job)
        }
        self
    }

    pub fn start(self) {
        for cronjob in self.cronjobs{
            cronjob.run()
        }
    }
}