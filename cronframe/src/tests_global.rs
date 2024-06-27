use crate::cronframe::CronFilter;
use crate::distributed_slice;
use crate::{Any, Arc, CronFrame, JobBuilder};
use chrono::{DateTime, Duration, Local, Timelike, Utc};
use cronframe_macro::{cron, cron_impl, cron_obj, job};
use std::fs;

#[test]
fn global_job() {
    #[cron(expr = "0/5 * * * * * *", timeout = "0")]
    fn testfn() {
        println!("call from testfn");
    }

    let file_path = "log/latest.log";
    let cronframe = CronFrame::init(Some(CronFilter::Global));

    // execute for a given time
    let mut first_run: DateTime<Utc> = cronframe.cron_jobs.lock().unwrap()[0]
        .upcoming()
        .parse()
        .unwrap();

    cronframe.scheduler();

    println!("First Run = {first_run}");

    let start_time = Utc::now();
    let duration = Duration::seconds(15);
    let end_time = start_time + duration;

    println!("difference = {}", first_run - start_time);
    if first_run - start_time <= Duration::milliseconds(500) {
        println!("OLD First Run = {first_run}");
        first_run = first_run + Duration::seconds(5);
        println!("NEW First Run = {first_run}");
    }

    println!("START TIME IS: {start_time}");
    println!("END TIME IS: {end_time}");

    // make the lib execute for given time
    while end_time > Utc::now() {}

    // we need to get the current log file
    // if we don't have it, test fails
    let file_content = fs::read_to_string(file_path);
    assert_eq!(file_content.is_ok(), true);
    let file_content = file_content.unwrap();

    // if we have the file content then we check its contents
    // the first check is to see if there are executions
    assert!(
        file_content.contains("Execution"),
        "no execution in the log file"
    );

    // then we check that the time difference between each execution is 5 seconds
    let lines = file_content.lines();
    let mut exec_times = Vec::new();
    for line in lines {
        if line.contains("Execution") {
            let time = (&line[..26]).to_owned();
            println!("{time} : str");
            let time: DateTime<Utc> = time.parse().unwrap();
            println!("{time} : datetime");
            exec_times.push(time);
        }
    }

    let duration = Duration::seconds(5);

    // check the first run of the job matches the expected time
    assert!(first_run == exec_times[0]);

    // check all following runs follow the defined interval
    for i in 0..(exec_times.len() - 1) {
        assert!(
            exec_times[i] + duration == exec_times[i + 1],
            "execution time interval error"
        );
    }
}
