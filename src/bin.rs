use cronframe::{CronFilter, CronFrame};
use std::{
    path::Path,
    process::{Command, Stdio},
};

fn main() {
    println!("CronFrame CLI Tool");

    let home_dir = {
        let tmp = home::home_dir().unwrap();
        tmp.to_str().unwrap().to_owned()
    };

    let main_arg = std::env::args().nth(1).expect("arg required");

    if main_arg == "start" {
        let _build = Command::new("cronframe")
            .args(["run"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("cronframe run failed");
        println!("CronFrame will soon be available at: http://localhost:8098");
    }
    if main_arg == "run" {
        let _ = CronFrame::init(Some(CronFilter::CLI), true).run();
    } else if main_arg == "shutdown" {
        let req_url = format!("http://localhost:8098/shutdown");

        match reqwest::blocking::get(req_url) {
            Ok(_) => {
                println!("CronFrame will soon shutdown.");
            }
            Err(error) => {
                println!("Error:");
                println!("{error}");
            }
        }
    } else if main_arg == "add" {
        let expr = std::env::args().nth(2).expect("expr is required");
        let timeout = std::env::args().nth(3).expect("expr is required");
        let job = std::env::args().nth(4).expect("job is required");

        let escaped_expr = expr.replace("/", "slh");

        let job_name = if Path::new(&job).is_file() {
            let tmp: Vec<_> = job.split("/").collect();
            let job_name = tmp.last().unwrap().replace(".rs", "");

            // compile the "script" job
            let _ = Command::new("rustc")
                .args([&job, "-o", &format!("{home_dir}/.cronframe/jobs/{job_name}")])
                .status()
                .expect("job compilation failed");

            job_name
        } else {
            let _ = Command::new("cargo")
            .args(["build"])
            .current_dir(&job)
            .status()
            .expect("job compilation failed");

            "placeholder".into()
        };

        // // compile the "crate" job
        // let _build = Command::new("cargo")
        //     .current_dir(&job)
        //     .args(["build", "--release"])
        //     .status()
        //     .expect("job compilation failed");

        // send the job to the running cronframe instance
        // localhost::8098/add_cli_job/<expr>/<timeout>/<job>
        let req_url = format!("http://localhost:8098/add_cli_job/{escaped_expr}/{timeout}/{job_name}");

        match reqwest::blocking::get(req_url) {
            Ok(_) => {
                println!("Added Job to CronFrame");
                println!("\tName: {job_name}");
                println!("\tCron Expression: {expr}");
                println!("\tTimeout: {timeout}");
            }
            Err(error) => {
                println!("Error adding a Job to CronFrame");
                println!("{error}");
            }
        }
    }

    // let _build = Command::new("cronframe").status().expect("process failed to execute");

    // let cronframe = CronFrame::init(Some(CronFilter::CLI), true).add_job(
    //     JobBuilder::cli_job("cli_job", "0/5 * * * * * *", "0").build()
    // );

    // cronframe.run();

    // let _build = Command::new("cargo")
    //     .current_dir("examples/weather_alert")
    //     .args(["build", "--release"])
    //     .status()
    //     .expect("process failed to execute");

    // // run a job form crate
    // let _run_job = Command::new("./weather_alert")
    //     .current_dir("examples/weather_alert/target/release")
    //     .status()
    //     .expect("process failed to execute");

    // run a job form standalone file
    // let _run_job = Command::new("./cli_job")
    //     .current_dir("examples/")
    //     .status()
    //     .expect("process failed to execute");
}
