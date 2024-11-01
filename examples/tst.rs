#[macro_use]
extern crate cronframe;
use cronframe::{ConfigData, CronFrame, LoggerConfig, SchedulerConfig, ServerConfig};

#[cron(expr = "0/5 * * * * * *", timeout = "0")]
fn general_greeting_job() {
    println!("Have a good morning!");
}

fn main() {
    let _config = ConfigData {
        webserver: ServerConfig::default(),
        logger: LoggerConfig::default(),
        scheduler: SchedulerConfig::default(),
    };

    //let cronframe = CronFrame::with_config(_config).unwrap();
    let cronframe = CronFrame::init().unwrap();

    println!("CronFilter is {:?}", cronframe.job_filter());
    cronframe.run();
}
