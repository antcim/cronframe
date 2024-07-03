#![allow(warnings)]

#[macro_use]
extern crate cronframe;
use core::panic;

use cronframe::{Any, Arc, CronFrame, JobBuilder};

//  Cron Expression
//  * * * * * * *
//  | | | | | | |
//  | | | | | | └─ year
//  | | | | | └─── day of week (1 to 7 for Sunday to Saturday, or three letter day)
//  | | | | └───── month (1 to 12 or 3 letter month like Jen, Feb, Mar, ...)
//  | | | └─────── day of month (1 to 31)
//  | | └───────── hours (0 to 23)
//  | └─────────── minutes (0 to 59)
//  └───────────── seconds (0 to 59)
// "*" works as a jolly for any value will do

#[cron(expr = "0/5 * * * * * *", timeout = "0")]
fn testfn() {
    println!("call from testfn");
}

#[cron(expr = "0/30 * * * * * *", timeout = "60000")]
fn another_test() {
    println!("call from another_test");
}

#[cron(expr = "0/30 * * * * * *", timeout = "0")]
fn heavy_job() {
    let mut _count: i128 = 0;

    for i in 0..5000000000 {
        _count += i;
    }
}

#[cron(expr = "0/5 * * * * * *", timeout = "0")]
fn failing_job() {
    panic!()
}

#[derive(Debug, Clone)]
#[cron_obj]
struct Users {
    second: String,
    minute: String,
    hour: String,
    day_month: String,
    month: String,
    day_week: String,
    year: String,
    timeout: u64,
}

#[cron_impl]
impl Users {
    #[job(expr = "0 0 * * * * *", timeout = "10000")]
    fn my_obj_job() {
        println!("call from my_obj_job");
    }
    
    #[job]
    fn get_jobs(self) {
        println!("call from get_jobs for seconds {}", self.second);
    }
}

fn main() {
    let cronframe = CronFrame::default();

    let user1 = Users {
        second: "0/5".to_string(),
        minute: "*".to_string(),
        hour: "*".to_string(),
        day_month: "*".to_string(),
        month: "*".to_string(),
        day_week: "*".to_string(),
        year: "*".to_string(),
        timeout: 0,
    };

    let user2 = Users {
        second: "0/10".to_string(),
        minute: "*".to_string(),
        hour: "*".to_string(),
        day_month: "*".to_string(),
        month: "*".to_string(),
        day_week: "*".to_string(),
        year: "*".to_string(),
        timeout: 0,
    };

    user1.helper_gatherer(cronframe.clone());
    user2.helper_gatherer(cronframe.clone());

    cronframe.scheduler();

    loop {
        println!("Enter x to quit...");
        let mut user_input: String = String::new();
        std::io::stdin()
            .read_line(&mut user_input)
            .expect("Error on user input read!");

        match user_input.trim() {
            "x" => break,
            _ => println!("invalid input"),
        }
    }
}
