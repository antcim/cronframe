// cronframe framework example
#[macro_use]
extern crate cronframe;

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

#[cron_obj]
struct JobContainer;

#[cron_impl]
impl JobContainer {
    #[fn_job(expr = "0/5 * * * * * *", timeout = "0")]
    fn my_function_job_1() {
        println!("call from my_function_job_1");
    }

    #[fn_job(expr = "0/10 * * * * * *", timeout = "30000")]
    fn my_function_job_2() {
        println!("call from my_function_job_2");
    }

    #[fn_job(expr = "0/15 * * * * * *", timeout = "60000")]
    fn my_function_job_3() {
        println!("call from my_function_job_3");
    }
}

fn main() {
    let cronframe = CronFrame::init().unwrap().start_scheduler();

    JobContainer::cf_gather_fn(cronframe.clone());

    cronframe.run();
}
