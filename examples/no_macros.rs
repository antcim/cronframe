// cronframe framework example
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

fn useless_job() {
    println!("not doing much...");
}

fn main() {
    CronFrame::init()
        .unwrap()
        .new_job("hello_job", || println!("hello job"), "* * * * * * *", "0")
        .new_job("useless_job", useless_job, "0/5 * * * * * *", "0")
        .run();
}
