#[macro_use] extern crate cronframe;

use std::time::Duration;
use cronframe::CronFrame;

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
fn not_doing_much() {
    println!("not_doing_much");
}

fn useless_job() {
    println!("not doing much...");
}

fn wait_seconds(seconds: u64){
    std::thread::sleep(Duration::from_secs(seconds));
}

fn main() {
    let cf = CronFrame::default()
        .new_job("hello_job", || println!("hello job"), "* * * * * * *", "0")
        .new_job("useless_job", useless_job, "0/5 * * * * * *", "0");

    wait_seconds(15);

    println!("STARTING SCHEDULER");
    cf.start_scheduler();

    wait_seconds(15);

    println!("STOPPING SCHEDULER");
    cf.stop_scheduler();
    println!("SCHEDULER STOPPED");

    wait_seconds(15);

    println!("STARTING SCHEDULER AGAIN");
    cf.run();
}
