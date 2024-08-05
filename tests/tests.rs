use std::{fs, ptr::addr_of_mut, sync::Once};

use chrono::{DateTime, Duration, Utc};
use cronframe::{logger, CronFilter, CronFrame, CronFrameExpr, JobBuilder};
use cronframe_macro::{cron, cron_impl, cron_obj, fn_job, mt_job};

static LOGGER_INIT: Once = Once::new();

static mut LOGGER: Option<log4rs::Handle> = None;

#[cron(expr = "0/5 * * * * * *", timeout = "0")]
fn my_global_job_std() {
    println!("call from global job standard");
}

#[cron(expr = "0/5 * * * * * *", timeout = "15000")]
fn my_global_job_timeout() {
    println!("call from global job with timeout");
}

#[cron(expr = "0/5 * * * * * *", timeout = "0")]
fn my_global_job_fail() {
    println!("call from global job with failure");
    panic!();
}

#[cron_obj]
#[derive(Debug, Clone)]
struct FunctionStd;

#[cron_impl]
impl FunctionStd {
    // this job executes every minute
    #[fn_job(expr = "0 * * * * *", timeout = "0")]
    fn my_function_job_std() {
        println!("call from function job");
    }
}

#[cron_obj]
#[derive(Debug, Clone)]
struct FunctionTimeout;

#[cron_impl]
impl FunctionTimeout {
    // this job executes every minute but quits after 3 minutes
    #[fn_job(expr = "0 * * * * *", timeout = "180000")]
    fn my_function_job_timeout() {
        println!("call from function job with timeout");
    }
}

#[cron_obj]
#[derive(Debug, Clone)]
struct FunctionFail;

#[cron_impl]
impl FunctionFail {
    // this job executes every minute
    #[fn_job(expr = "0 * * * * *", timeout = "0")]
    fn my_function_job_fail() {
        println!("call from function job with fail");
        panic!();
    }
}

#[cron_obj]
#[derive(Debug, Clone)]
struct MethodStd {
    expr: CronFrameExpr,
}

#[cron_impl]
impl MethodStd {
    #[mt_job(expr = "expr")]
    fn my_method_job_std(self) {
        println!("call from method_job");
    }
}

#[cron_obj]
#[derive(Debug, Clone)]
struct MethodTimeout {
    expr: CronFrameExpr,
}

#[cron_impl]
impl MethodTimeout {
    #[mt_job(expr = "expr")]
    fn my_method_job_timeout(self) {
        println!("call from method_job_timeout");
    }
}

#[cron_obj]
#[derive(Debug, Clone)]
struct MethodFail {
    expr: CronFrameExpr,
}

#[cron_impl]
impl MethodFail {
    #[mt_job(expr = "expr")]
    fn my_method_job_fail(self) {
        println!("call from method_job with fail");
        panic!();
    }
}

pub fn init_logger(path: &str) {
    LOGGER_INIT.call_once(|| {
        unsafe { LOGGER = Some(logger::appender_logger("log/latest.log")) };
        let _ = std::fs::remove_file("log/latest.log");
    });

    unsafe {
        if let Some(handle) = addr_of_mut!(LOGGER).as_ref().unwrap() {
            handle.set_config(logger::appender_config(path))
        }
    }
}

pub fn test_job(
    file_path: &str,
    job_filter: CronFilter,
    job_name: &str,
    duration: Duration,
    interval: Duration,
    timeout: Duration,
    should_fail: bool,
) {
    init_logger(file_path);

    let cronframe = CronFrame::init(Some(job_filter), false);

    let expr_fail = CronFrameExpr::new("0", "0/5", "*", "*", "*", "*", "*", 0);
    let expr_timeout = CronFrameExpr::new("0", "*/5", "*", "*", "*", "*", "*", 720000);
    let expr_std = CronFrameExpr::new("0", "0/5", "*", "*", "*", "*", "*", 0);

    let mut mt_fail = MethodFail::new_cron_obj(expr_fail);
    let mut mt_timeout = MethodTimeout::new_cron_obj(expr_timeout);
    let mut mt_std = MethodStd::new_cron_obj(expr_std);

    match job_filter {
        CronFilter::Function => {
            if should_fail {
                FunctionFail::cf_gather_fn(cronframe.clone());
            } else if timeout > Duration::seconds(0) {
                FunctionTimeout::cf_gather_fn(cronframe.clone());
            } else {
                FunctionStd::cf_gather_fn(cronframe.clone());
            }
        }
        CronFilter::Method => {
            if should_fail {
                mt_fail.cf_gather(cronframe.clone());
            } else if timeout > Duration::seconds(0) {
                mt_timeout.cf_gather(cronframe.clone());
            } else {
                mt_std.cf_gather(cronframe.clone());
            }
        }
        _ => (), // no additional stuff to do if global job
    }

    // execute for a given time
    let mut first_run: DateTime<Utc> = cronframe
        .cron_jobs
        .lock()
        .unwrap()
        .iter()
        .find(|job| job.name.contains(job_name))
        .unwrap()
        .upcoming_utc()
        .parse()
        .unwrap();

    cronframe.start_scheduler();

    println!("First Run = {first_run}");

    let start_time = Utc::now();
    let end_time = start_time + duration;

    println!("difference = {}", first_run - start_time);
    if first_run - start_time <= Duration::milliseconds(500) {
        println!("OLD First Run = {first_run}");
        first_run = first_run + interval;
        println!("NEW First Run = {first_run}");
    }

    println!("START TIME IS: {start_time}");
    println!("END TIME IS: {end_time}");

    // make the lib execute for given time
    while end_time > Utc::now() {}
    cronframe.quit();

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
        if line.contains(format!("{job_name} ").as_str()) {
            if line.contains("Execution") {
                let time_pieces: Vec<_> = line.split(" ").collect();
                let time = format!("{} {} {}", time_pieces[0], time_pieces[1], time_pieces[2]);
                println!("{time} : str");
                let time: DateTime<Utc> = time.parse().unwrap();
                println!("{time} : datetime");
                exec_times.push(time);
            } else if line.contains("Timeout") {
                let time_pieces: Vec<_> = line.split(" ").collect();
                let time = format!("{} {} {}", time_pieces[0], time_pieces[1], time_pieces[2]);
                let time: DateTime<Utc> = time.parse().unwrap();
                timeouts.push(time);
            }
        }
    }

    // check the first run of the job matches the expected time
    assert!(first_run == exec_times[0]);

    // check all following runs follow the defined interval
    for i in 0..(exec_times.len() - 1) {
        assert!(
            exec_times[i] + interval == exec_times[i + 1],
            "execution time interval error"
        );
    }

    if timeout > Duration::seconds(0) {
        // timeout is counted in the scheduler from the moment a job thread is spawned for the first time
        // a job thread is spawned at least 500ms before its upcoming schedule
        // so here we account for the previous second pertaining the scheduling and not the execution
        let timeout = timeout - Duration::seconds(1);
        let grace = Duration::milliseconds(cronframe.grace.into());
        assert!(
            (first_run + timeout + grace <= timeouts[0]) || (first_run + timeout >= timeouts[0]),
            "timeout error"
        );
    }

    if should_fail {
        assert!(
            file_content.contains("Aborted"),
            "No abortion in the log file."
        );
    }
}

mod global {
    use crate::{test_job, CronFilter};
    use chrono::Duration;

    #[test]
    fn global_job_std() {
        let file_path = "log/global_job_std.log";
        let job_filter = CronFilter::Global;
        let job_name = "my_global_job_std";
        let duration = Duration::seconds(15);
        let interval = Duration::seconds(5);
        let timeout = Duration::seconds(0);
        let should_fail = false;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }

    #[test]
    fn global_job_timeout() {
        let file_path = "log/global_job_timeout.log";
        let job_filter = CronFilter::Global;
        let job_name = "my_global_job_timeout";
        let duration = Duration::seconds(30);
        let interval = Duration::seconds(5);
        let timeout = Duration::seconds(15);
        let should_fail = false;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }

    #[test]
    fn global_job_fail() {
        let file_path = "log/global_job_fail.log";
        let job_filter = CronFilter::Global;
        let job_name = "my_global_job_fail";
        let duration = Duration::seconds(15);
        let interval = Duration::seconds(5);
        let timeout = Duration::seconds(0);
        let should_fail = true;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }
}

mod function {
    use crate::{test_job, CronFilter};
    use chrono::Duration;

    #[test]
    fn function_job_std() {
        let file_path = "log/function_job_std.log";
        let job_filter = CronFilter::Function;
        let job_name = "my_function_job_std";
        let duration = Duration::minutes(5);
        let interval = Duration::minutes(1);
        let timeout = Duration::seconds(0);
        let should_fail = false;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }

    #[test]
    fn function_job_timeout() {
        let file_path = "log/function_job_timeout.log";
        let job_filter = CronFilter::Function;
        let job_name = "my_function_job_timeout";
        let duration = Duration::minutes(5);
        let interval = Duration::minutes(1);
        let timeout = Duration::minutes(3);
        let should_fail = false;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }

    #[test]
    fn function_job_fail() {
        let file_path = "log/function_job_fail.log";
        let job_filter = CronFilter::Function;
        let job_name = "my_function_job_fail";
        let duration = Duration::minutes(5);
        let interval = Duration::minutes(1);
        let timeout = Duration::seconds(0);
        let should_fail = true;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }
}

mod method {
    use crate::{test_job, CronFilter};
    use chrono::Duration;

    #[test]
    fn method_job_std() {
        let file_path = "log/method_job_std.log";
        let job_filter = CronFilter::Method;
        let job_name = "my_method_job_std";
        let duration = Duration::minutes(15);
        let interval = Duration::minutes(5);
        let timeout = Duration::minutes(0);
        let should_fail = false;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }

    #[test]
    fn method_job_timeout() {
        let file_path = "log/method_job_timeout.log";
        let job_filter = CronFilter::Method;
        let job_name = "my_method_job_timeout";
        let duration = Duration::minutes(20);
        let interval = Duration::minutes(5);
        let timeout = Duration::minutes(12);
        let should_fail = false;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }

    #[test]
    fn method_job_fail() {
        let file_path = "log/method_job_fail.log";
        let job_filter = CronFilter::Method;
        let job_name = "my_method_job_fail";
        let duration = Duration::minutes(15);
        let interval = Duration::minutes(5);
        let timeout = Duration::seconds(0);
        let should_fail = true;

        test_job(
            file_path,
            job_filter,
            job_name,
            duration,
            interval,
            timeout,
            should_fail,
        );
    }
}
