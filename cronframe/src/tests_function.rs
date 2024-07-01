use crate::cronframe::CronFilter;
use crate::tests::init_logger;
use crate::{distributed_slice, logger};
use crate::{Any, Arc, CronFrame, JobBuilder};
use chrono::{DateTime, Duration, Local, Timelike, Utc};
use cronframe_macro::{cron, cron_impl, cron_obj, job};
use std::fs;

#[derive(Debug, Clone)]
#[cron_obj]
struct TestingStr {
    second: String,
    minute: String,
    hour: String,
    day_month: String,
    month: String,
    day_week: String,
    year: String,
    timeout: u64,
}

#[cron_impl]
impl TestingStr {
    // this job executes every minute
    #[job(expr = "0 * * * * *", timeout = "0")]
    fn my_function_job_std() {
        println!("call from function job");
    }
    // this job executes every minute but quits after 3 minutes
    #[job(expr = "0 * * * * *", timeout = "180000")]
    fn my_function_job_timeout() {
        println!("call from function job with timeout");
    }
}

#[test]
fn function_job_std() {
    init_logger();

    let file_path = "log/latest.log";
    let cronframe = CronFrame::init(Some(CronFilter::Function), false);

    let user1 = TestingStr {
        second: String::default(),
        minute: String::default(),
        hour: String::default(),
        day_month: String::default(),
        month: String::default(),
        day_week: String::default(),
        year: String::default(),
        timeout: 0,
    };
    user1.helper_gatherer(cronframe.clone());

    // execute for a given time
    let mut first_run: DateTime<Utc> = cronframe
        .cron_jobs
        .lock()
        .unwrap()
        .iter()
        .find(|job| job.name.contains("my_function_job_std"))
        .unwrap()
        .upcoming()
        .parse()
        .unwrap();

    cronframe.scheduler();

    println!("First Run = {first_run}");

    let start_time = Utc::now();
    let duration = Duration::minutes(5);
    let end_time = start_time + duration;

    println!("difference = {}", first_run - start_time);
    if first_run - start_time <= Duration::milliseconds(500) {
        println!("OLD First Run = {first_run}");
        first_run = first_run + Duration::minutes(1);
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
        if line.contains("my_function_job_std ") {
            if line.contains("Execution") {
                let time = (&line[..26]).to_owned();
                println!("{time} : str");
                let time: DateTime<Utc> = time.parse().unwrap();
                println!("{time} : datetime");
                exec_times.push(time);
            }
        }
    }

    let duration = Duration::seconds(60);

    // check the first run of the job matches the expected time
    assert!(first_run == exec_times[0]);

    // check all following runs follow the defined interval
    for i in 0..(exec_times.len() - 1) {
        assert!(
            exec_times[i] + duration == exec_times[i + 1],
            "execution time interval error"
        );
    }

    cronframe.quit();
}

#[test]
fn function_job_timeout() {
    init_logger();

    let file_path = "log/latest.log";
    let cronframe = CronFrame::init(Some(CronFilter::Function), false);

    let user1 = TestingStr {
        second: String::default(),
        minute: String::default(),
        hour: String::default(),
        day_month: String::default(),
        month: String::default(),
        day_week: String::default(),
        year: String::default(),
        timeout: 0,
    };
    user1.helper_gatherer(cronframe.clone());

    // execute for a given time
    let mut first_run: DateTime<Utc> = cronframe
        .cron_jobs
        .lock()
        .unwrap()
        .iter()
        .find(|job| job.name.contains("my_function_job_timeout"))
        .unwrap()
        .upcoming()
        .parse()
        .unwrap();

    cronframe.scheduler();

    println!("First Run = {first_run}");

    let start_time = Utc::now();
    let duration = Duration::minutes(5);
    let end_time = start_time + duration;

    println!("difference = {}", first_run - start_time);
    if first_run - start_time <= Duration::milliseconds(500) {
        println!("OLD First Run = {first_run}");
        first_run = first_run + Duration::minutes(1);
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
    let mut timeouts = Vec::new();
    for line in lines {
        if line.contains("my_function_job_timeout ") {
            if line.contains("Execution") {
                let time = (&line[..26]).to_owned();
                //println!("{time} : str");
                let time: DateTime<Utc> = time.parse().unwrap();
                println!("{time} : datetime");
                exec_times.push(time);
            } else if line.contains("Timeout") {
                let time = (&line[..26]).to_owned();
                let time: DateTime<Utc> = time.parse().unwrap();
                timeouts.push(time);
            }
        }
    }

    let duration = Duration::seconds(60);
    // timeout here is actually 3 seconds
    // but it is counted in the scheduler from the moment a job thread is spawned
    // a job thread is spawned at least 500ms before its scheduled execution
    // so here we account for the previous second pertaining the scheduling and not the execution
    let timeout = Duration::seconds(179);

    // check the first run of the job matches the expected time
    assert!(first_run == exec_times[0]);

    // check all following runs follow the defined interval
    for i in 0..(exec_times.len() - 1) {
        assert!(
            exec_times[i] + duration == exec_times[i + 1],
            "execution time interval error"
        );
    }

    assert!(first_run + timeout == timeouts[0], "timeout error");

    cronframe.quit();
}

