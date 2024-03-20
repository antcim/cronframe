#[macro_use]
extern crate cronlib;

#[cron("* * * * *")]
fn testfn() {
    println!("test");
}

fn main() {
    println!("CronFrame 0.0.1");
    testfn();
    testfn_aux_1();
    testfn_aux_2();
}
