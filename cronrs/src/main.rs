use cronlib::{CronFrame, *};

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
#[cron(expr = "0/5 * * * * * *", timeout = "10000")]
fn testfn() {
    println!("call from testfn");
}

// this executes once every day of every month at 13:30 UTC time
// #[cron(expr = "0 30 13 * * *", timeout = "2000")]
// fn myjob() {
//     println!("call from myjob!!!");
// }

fn main() {
    println!("CronFrame 0.0.1");
    CronFrame::init().schedule();
}
