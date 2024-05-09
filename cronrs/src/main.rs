use cronlib::{CronFrame, *};
use std::io;

//  Cron Expression
//  * * * * * * *
//  | | | | | | |
//  | | | | | | └─ year
//  | | | | | └─── day of week (0 to 7, Sunday to Saturday, 0 and 7 both work for Sunday or three letter day)
//  | | | | └───── month (1 to 12 or 3 letter month like JEN, FEB, MAR,...)
//  | | | └─────── day of month (1 to 31)
//  | | └───────── hour (0 to 23)
//  | └─────────── minute (0 to 59)
//  └───────────── second (0 to 59)
// "*" works as a jolly for any value will do

// this executes every 5 seconds
#[cron(expr = "0/5 * * * * * *", timeout = "0")]
fn testfn() {
    println!("call from testfn");
}

//this executes once every day of every month at 13:30 UTC time
#[cron(expr = "0 9 8 * * *", timeout = "0")]
fn myjob() {
    println!("call from myjob!!!");
}

fn main() {
    println!("CronFrame 0.0.1");
    CronFrame::init().scheduler();

    println!("Enter x to quit...");
    let mut user_input: String = String::new();

    loop {
        io::stdin()
            .read_line(&mut user_input)
            .expect("Error on user input read!");

        match user_input.trim() {
            "x" => break,
            _ => println!("invalid input"),
        }
    }
}
