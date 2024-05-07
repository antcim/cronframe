pub use cronmacro::cron;
pub use cron::Schedule;
pub use std::str::FromStr;
pub use std::thread;
pub use chrono::{Utc, Duration};
use crossbeam_channel::{Receiver, Sender};
use std::{thread::JoinHandle, vec};

// necessary to gather all the annotated jobs automatically
inventory::collect!(CronJob);

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

    pub fn run(&self, rx: Receiver<String>) -> JoinHandle<()> {
        let job = self.job.clone();
        let schedule = (self.get_info)().0;

        let job_thread = move ||{
            loop {
                match rx.try_recv() {
                    Ok(message) => {
                        if message == "EXIT_TIMEOUT"{
                            println!("EXIT DUE TO TIMEOUT");
                            break
                        }
                    },
                    Err(_error) => (),
                }

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
    channels: Vec<(Sender<String>, Receiver<String>)>,
    start_times: Vec<chrono::DateTime<Utc>>,
    timeouts: Vec<Option<chrono::DateTime<Utc>>>,
}

impl<'a> CronFrame<'a>{
    pub fn init() -> Self{
        let mut frame = CronFrame {
            cronjobs: vec![],
            handlers: vec![],
            channels: vec![],
            start_times: vec![],
            timeouts: vec![],
        };

        // get the automatically collected jobs  
        for job in inventory::iter::<CronJob> {
            frame.cronjobs.push(job)
        }

        frame
    }

    pub fn schedule(mut self) {
        for cronjob in &self.cronjobs{
            let channels = crossbeam_channel::unbounded();
            let handler = cronjob.run(channels.1.clone());

            let timeout_ms = chrono::Duration::milliseconds((cronjob.get_info)().1);
            let start_time = Utc::now();
            let timeout = start_time + timeout_ms;

            self.channels.push(channels);
            self.handlers.push(handler);
            self.start_times.push(start_time);

            if start_time == timeout{
                self.timeouts.push(None);
            }else{
                self.timeouts.push(Some(timeout));
            }
        }

        loop{
            let mut to_remove = vec![];

            for (i, (tx, _)) in self.channels.iter().enumerate(){
                if let Some(timeout) = self.timeouts[i]{
                    let now = Utc::now();
                    if now >= timeout{
                        let _ = tx.send("EXIT_TIMEOUT".to_string());
                        to_remove.push(i);
                    }
                }
            }

            let mut count = 0;
            for i in to_remove{
                self.cronjobs.remove(i - count);
                self.handlers.remove(i - count);
                self.start_times.remove(i - count);
                self.channels.remove(i - count);
                self.timeouts.remove(i - count);
                count += 1;
            }

            if self.handlers.len() == 0{
                break
            }
        }
    }
}