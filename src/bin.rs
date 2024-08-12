use cronframe::{utils, CronFilter, CronFrame};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

fn main() {
    let home_dir = utils::home_dir();
    
    println!("CronFrame CLI Tool");
    
    if !std::path::Path::new(&format!("{home_dir}/.cronframe")).exists() {
        println!("Generating .cronframe directory content...");
        fs::create_dir(format!("{home_dir}/.cronframe"))
            .expect("could not create .cronframe directory");
        fs::create_dir(format!("{home_dir}/.cronframe/jobs"))
            .expect("could not create .cronframe directory");
    }

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

        let tmp: Vec<_> = job.split("/").collect();
        let job_name = tmp.last().unwrap().replace(".rs", "");

        if Path::new(&job).is_file() {
            // compile the "script" job
            let tmp: Vec<_> = job.split("/").collect();
            let job_name = tmp.last().unwrap().replace(".rs", "");

            let _ = Command::new("rustc")
                .args([
                    &job,
                    "-o",
                    &format!("{home_dir}/.cronframe/jobs/{job_name}"),
                ])
                .status()
                .expect("job compilation failed");
        } else {
            // compile the "crate" job
            let _ = Command::new("cargo")
                .args([
                    "build",
                    "--release",
                    "--target-dir",
                    &format!("{home_dir}/.cronframe/cargo_targets/{job_name}"),
                ])
                .current_dir(job)
                .status()
                .expect("job compilation failed");

            let _ = Command::new("cp")
                .args([&job_name, &format!("{home_dir}/.cronframe/jobs/{job_name}")])
                .current_dir(format!(
                    "{home_dir}/.cronframe/cargo_targets/{job_name}/release"
                ))
                .status()
                .expect("job compilation failed");
        }

        // send the job to the running cronframe instance
        // localhost::8098/add_cli_job/<expr>/<timeout>/<job>
        let req_url =
            format!("http://localhost:8098/add_cli_job/{escaped_expr}/{timeout}/{job_name}");

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
}
