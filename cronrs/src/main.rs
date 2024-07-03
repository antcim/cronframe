#![allow(warnings)]

#[macro_use]
extern crate cronframe;
use core::panic;

use cronframe::{Any, Arc, CronFrame, CronFrameExpr, JobBuilder};

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
    expr: CronFrameExpr,
    expr1: CronFrameExpr,
}

#[cron_impl]
impl Users {
    #[fn_job(expr = "0 0 * * * * *", timeout = "10000")]
    fn my_function_job() {
        println!("call from my_obj_job");
    }
    
    #[mt_job(expr = "expr")]
    fn my_method_job_1(self) {
        println!("call from my_method_job_1 for expr {}", self.expr.expr());
    }

    #[mt_job(expr = "expr1")]
    fn my_method_job_2(self) {
        println!("call from get_jobs for expr {}", self.expr1.expr());
    }
}

fn main() {
    let cronframe = CronFrame::default();

    let expr1 = CronFrameExpr::new("0/5", "*", "*", "*", "*", "*", "*", 0);
    let expr2 = CronFrameExpr::new("0/10", "*", "*", "*", "*", "*", "*", 20000);
    let expr3 = CronFrameExpr::new("0/7", "*", "*", "*", "*", "*", "*", 10000);

    let user1 = Users {
        expr: expr1,
        expr1: expr3.clone()
    };

    let user2 = Users {
        expr: expr2,
        expr1: expr3
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
