#![allow(warnings)]

#[macro_use]
extern crate cronframe;
use core::panic;

use chrono::Duration;
use cronframe::{Any, Arc, CronFrame, CronFrameExpr, JobBuilder, Sender};

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

// #[cron(expr = "0/5 * * * * * *", timeout = "0")]
// fn testfn() {
//     println!("call from testfn");
// }

// #[cron(expr = "0/30 * * * * * *", timeout = "60000")]
// fn another_test() {
//     println!("call from another_test");
// }

// #[cron(expr = "0/30 * * * * * *", timeout = "0")]
// fn heavy_job() {
//     let mut _count: i128 = 0;

//     for i in 0..5000000000 {
//         _count += i;
//     }
// }

// #[cron(expr = "0/5 * * * * * *", timeout = "0")]
// fn failing_job() {
//     panic!()
// }


#[cron_obj]
#[derive(Clone, Default)] // these traits are required
struct Users {
    name: String,
    expr: CronFrameExpr,
    expr1: CronFrameExpr,
}

#[cron_impl]
impl Users {
    // #[fn_job(expr = "0 0 * * * * *", timeout = "10000")]
    // fn my_function_job() {
    //     println!("call from my_obj_job");
    // }
    
    #[mt_job(expr = "expr")]
    fn my_method_job_1(self) {
        println!("call from my_method_job_1 for expr {}", self.expr.expr());
    }

    #[mt_job(expr = "expr1")]
    fn my_method_job_2(self) {
        println!("call from my_method_job_2 for expr {}", self.expr1.expr());
    }
}

fn main() {
    let cronframe = CronFrame::default().scheduler();
    std::thread::sleep(Duration::seconds(5).to_std().unwrap());

    let expr1 = CronFrameExpr::new("0/5", "*", "*", "*", "*", "*", "*", 0);
    let expr2 = CronFrameExpr::new("0/10", "*", "*", "*", "*", "*", "*", 20000);
    let expr3 = CronFrameExpr::new("0/7", "*", "*", "*", "*", "*", "*", 10000);

    let mut user1 = Users {
        name: "user1".to_string(),
        expr: expr1,
        expr1: expr3.clone(),
        tx: None,
    };

    user1.helper_gatherer(cronframe.clone());

    // inner scope to test the drop of cron_objects
    {
        let mut user2 = Users {
            name: "user2".to_string(),
            expr: expr2,
            expr1: expr3,
            tx: None,
        };

        user2.helper_gatherer(cronframe.clone());

        std::thread::sleep(Duration::seconds(15).to_std().unwrap());
    }

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
