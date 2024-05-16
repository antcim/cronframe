use cronlib::{CronFrame, *};
use std::io;

//  Cron Expression
//  * * * * * * *
//  | | | | | | |
//  | | | | | | └─ year
//  | | | | | └─── day of week (0 to 7, Sunday to Saturday, 0 and 7 both work for Sunday or three letter day)
//  | | | | └───── month (1 to 12 or 3 letter month like Jen, Feb, Mar,...)
//  | | | └─────── day of month (1 to 31)
//  | | └───────── hour (0 to 23)
//  | └─────────── minute (0 to 59)
//  └───────────── second (0 to 59), optional, defaults to every second "*"
// "*" works as a jolly for any value will do

// this executes every 5 seconds, timeouts after 10 seconds

#[cron(expr = "0/5 * * * * * *", timeout = "10000")]
fn testfn() {
    println!("call from testfn");
}

#[cron_obj] // this macro does nothing for now
struct Users {
    second: String,
    minute: String,
    hour: String,
    day_month: String,
    month: String,
    day_week: String,
    year: String,
}

#[cron_impl]
impl Users {
    #[job(expr = "2/5 * * * * * *", timeout = "0")]
    fn my_obj_job() {
        println!("call from my_obj_job");
    }
    #[job(expr = "0/5 * * * * * *", timeout = "10000")]
    fn get_jobs() {
        println!("call from get_jobs");
    }
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
