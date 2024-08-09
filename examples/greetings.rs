#[macro_use]
extern crate cronframe;
use cronframe::{CronFrame, CronFrameExpr};

#[cron_obj]
struct Greeting {
    employee: String,
    my_expr: CronFrameExpr,
}

#[cron_impl]
impl Greeting {
    #[fn_job(expr = "0 0 8 * * Mon-Fri *", timeout = "0")]
    fn general_greeting_job() {
        println!("Have a good morning!");
    }

    #[mt_job(expr = "my_expr")]
    fn specific_greeting_job(self) {
        println!("Hi {}, have a good morning!", self.employee);
    }
}

fn main() {
    let cronframe = CronFrame::default();

    let mut greeting_john = Greeting::new_cron_obj("John".into(), "0 0 18 * * Mon-Fri * 0".into());
    let mut greeting_jane =
        Greeting::new_cron_obj("Jane".into(), CronFrameExpr::from("0 0 16 * * Tue-Thu * 0"));

    greeting_john.cf_gather(cronframe.clone());
    greeting_jane.cf_gather(cronframe.clone());

    cronframe.run();
}
