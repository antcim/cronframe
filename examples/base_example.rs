#[macro_use] extern crate cronframe;

use chrono::Duration;
use core::panic;
use cronframe::{JobBuilder, CronFrameExpr, CronFrame};

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

    for i in 0..5_000_000_000 {
        _count += i;
    }
}

#[cron(expr = "0/5 * * * * * *", timeout = "0")]
fn failing_job() {
    panic!()
}

#[cron_obj]
#[derive(Clone)] // this trait is required
struct Users {
    name: String,
    expr: CronFrameExpr,
    expr1: CronFrameExpr,
}

#[cron_impl]
impl Users {
    #[fn_job(expr = "0/5 * * * * * *", timeout = "10000")]
    fn my_function_job_1() {
        println!("call from my_function_job_1");
    }

    #[fn_job(expr = "0/5 * * * * * *", timeout = "0")]
    fn my_function_job_2() {
        println!("call from my_function_job_2");
    }

    #[fn_job(expr = "0/8 * * * * * *", timeout = "20000")]
    fn my_function_job_3() {
        println!("call from my_function_job_3");
    }

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
    let cronframe = CronFrame::default().start_scheduler();

    let expr1 = CronFrameExpr::new("0/5", "*", "*", "*", "*", "*", "*", 0);
    let expr2 = CronFrameExpr::new("0/10", "*", "*", "*", "*", "*", "*", 20000);
    let expr3 = CronFrameExpr::new("0/7", "*", "*", "*", "*", "*", "*", 10000);

    // inner scope to test the drop of cron_objects
    {
        println!("PHASE 1");
        let mut user1 = Users::new_cron_obj("user1".to_string(), expr1.clone(), expr3.clone());

        user1.cf_gather(cronframe.clone());
        std::thread::sleep(Duration::seconds(10).to_std().unwrap());

        println!("PHASE 2");
        {
            let mut user2 = Users::new_cron_obj("user2".to_string(), expr2, expr3.clone());

            user2.cf_gather(cronframe.clone());

            std::thread::sleep(Duration::seconds(10).to_std().unwrap());
            user2.cf_drop();
        }
        user1.cf_drop();
    }

    println!("PHASE 3");

    std::thread::sleep(Duration::seconds(10).to_std().unwrap());

    println!("PHASE 4");

    let mut user3 = Users::new_cron_obj("user3".to_string(), expr1, expr3);
    user3.cf_gather(cronframe.clone());

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
