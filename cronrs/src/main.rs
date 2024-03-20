extern crate cronlib;
use cronlib::cron;

#[cron("* * * * *")]
fn testfn() {
    println!("Hello, world!");
}

fn main() {
    println!("CronFrame 0.0.1");
    testfn();
}