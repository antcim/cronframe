#[macro_use]
extern crate cronlib;
use cron::Schedule;
use std::str::FromStr;
use std::thread;
use chrono::Utc;

//  * * * * * *
//  | | | | | |
//  | | | | | └─── day of week (0 to 7, Sunday to Saturday, 0 and 7 both work for Sunday)
//  | | | | └───── month (1 to 12)
//  | | | └─────── day of month (1 to 31)
//  | | └───────── hour (0 to 23)
//  | └─────────── minute (0 to 59)
//  └───────────── second (0 to 59, optional)

#[cron("0/5 * * * * *")]
fn testfn() {
    println!("call from annotated function");
}

fn main() {
    println!("----------------");
    println!("CronFrame 0.0.1");
    println!("----------------");
    testfn_aux_1();
    testfn_aux_2();
}
