pub use cronmacro::cron;
pub use cron::Schedule;
pub use std::str::FromStr;
pub use std::thread;
pub use chrono::Utc;

// this crate ensures that both marco and  types it dependes on can be exported to public use
// necessary since proc-macro crates can only export with pub elements that have a #[proc...] annotation

pub struct CronFrame{
    functions: Vec<fn()>
}

impl CronFrame{
    pub fn init() -> Self{
        CronFrame {
            functions: vec![]
        }
    }

    pub fn schedule(mut self, functions: Vec<fn()>) -> Self{
        self.functions = functions;
        self 
    }

    pub fn start(self) {
        for f in self.functions{
            f()
        }
    }
}