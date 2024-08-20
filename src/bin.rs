//! CronFrame CLI Tool v0.1.3

use cronframe::{
    utils::{self, ip_and_port},
    web_server, CronFilter, CronFrame,
};
use std::{
    fs,
    io::BufRead,
    path::Path,
    process::{Command, Stdio},
};

use clap::{arg, command};

use colored::*;

fn main() {
    std::env::set_var("CRONFRAME_CLI", "true");

    // cli args parsing
    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("start")
                .about("Start the CronFrame Webserver and Job Scheduler in background."),
        )
        .subcommand(
            clap::Command::new("run")
                .about("Run the CronFrame Webserver and Job Scheduler in the terminal."),
        )
        .subcommand(
            clap::Command::new("add")
                .about("Adds a new cli job to a CronFrame instance.")
                .args(&[
                    arg!([EXPR] "The Cron Expression to use for job scheduling."),
                    arg!([TIMEOUT] "The value in ms to use for the timeout."),
                    arg!([JOB] "The path containing the source code of the job."),
                ])
                .arg_required_else_help(true)
                .arg(
                    arg!(-p --port <VALUE>)
                        .required(false)
                        .action(clap::ArgAction::Set),
                ),
        )
        .subcommand(
            clap::Command::new("load")
                .about("Load jobs from definition file.")
                .arg(
                    arg!(-f --file <PATH>)
                        .required(false)
                        .action(clap::ArgAction::Set),
                ),
        )
        .subcommand(
            clap::Command::new("scheduler")
                .about("Perform actions on the scheduler like start and stop")
                .args(&[arg!([ACTION] "Action to perform = (start, stop)")])
                .arg_required_else_help(true)
                .arg(
                    arg!(-p --port <VALUE>)
                        .required(false)
                        .action(clap::ArgAction::Set),
                ),
        )
        .subcommand(
            clap::Command::new("shutdown")
                .about("Shutdown the CronFrame Webserver and Job Scheduler."),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("start", _)) => start_command(),
        Some(("shutdown", _)) => shutdown_command(),
        Some(("run", _)) => run_command(),
        Some(("add", sub_matches)) => {
            let expr = sub_matches.get_one::<String>("EXPR").unwrap();
            let timeout = sub_matches.get_one::<String>("TIMEOUT").unwrap();
            let job = sub_matches.get_one::<String>("JOB").unwrap();
            let port_option = sub_matches.get_one::<String>("port");
            add_command(expr, timeout, job, port_option);
        }
        Some(("load", sub_matches)) => {
            let file = sub_matches.get_one::<String>("file");
            load_command(file);
        }
        Some(("scheduler", sub_matches)) => {
            let action = sub_matches.get_one::<String>("ACTION").unwrap();
            let port_option = sub_matches.get_one::<String>("port");
            scheduler_command(action, port_option);
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }
}

fn start_command() {
    cronframe_folder();

    let (ip, port) = ip_and_port();
    if is_running(&ip, port) {
        println!("{}", "Error when starting CronFrame".red().bold());
        println!("Address at: 'http://{ip}:{port}' is already busy");
        return;
    }

    let _build = Command::new("cronframe")
        .args(["run"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("cronframe run failed");

    println!("CronFrame will soon be available at: http://{ip}:{port}");
}

fn shutdown_command() {
    let (ip, port) = ip_and_port();
    let req_url = format!("http://{ip}:{port}/shutdown");

    match reqwest::blocking::get(req_url) {
        Ok(_) => {
            println!("CronFrame will soon shutdown.");
        }
        Err(_) => {
            println!("Error when shutting down CronFrame");
            println!("No instance found at: http://{ip}:{port}");
        }
    }
}

fn run_command() {
    cronframe_folder();
    let (ip, port) = ip_and_port();
    if is_running(&ip, port) {
        println!("{}", "Error when running CronFrame".red().bold());
        println!("Address at: 'http://{ip}:{port}' is already busy");
        return;
    }
    let _ = CronFrame::init(Some(CronFilter::CLI), true).run();
}

fn add_command(expr: &str, timeout: &str, job: &str, port_option: Option<&String>) {
    let home_dir = utils::home_dir().replace("\\", "/");

    let escaped_expr = expr.replace("/", "slh");

    let tmp: Vec<_> = if cfg!(target_os = "windows") {
        job.split("\\").collect()
    } else {
        job.split("/").collect()
    };
    let tmp = tmp.iter().filter(|x| !x.is_empty()); // needed if there is a / after the name of the create's folder
    let job_name = tmp.last().unwrap().replace(".rs", "");

    println!("Compiling {job_name} Job:");

    if Path::new(&job).is_file() {
        // compile the "script" job
        let _ = Command::new("rustc")
            .args([
                job,
                "-o",
                &format!("{home_dir}/.cronframe/cli_jobs/{job_name}"),
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

        let _ = if cfg!(target_os = "windows") {
            println!(
                "current dir = {}",
                format!("{home_dir}/.cronframe/cargo_targets/{job_name}/release")
            );
            println!(
                "cmd /C copy {} {}",
                format!("{job_name}.exe"),
                format!("{home_dir}/.cronframe/cli_jobs").replace("\\", "/")
            );

            Command::new("cmd")
                .args(&[
                    "/C",
                    "copy",
                    &format!("{job_name}.exe"),
                    &format!("{home_dir}/.cronframe/cli_jobs/").replace("/", "\\"),
                ])
                .current_dir(format!(
                    "{home_dir}/.cronframe/cargo_targets/{job_name}/release"
                ))
                .status()
                .expect("job binary copy failed")
        } else {
            // copy binary on unix systems
            Command::new("cp")
                .args([
                    &job_name,
                    &format!("{home_dir}/.cronframe/cli_jobs/{job_name}"),
                ])
                .current_dir(format!(
                    "{home_dir}/.cronframe/cargo_targets/{job_name}/release"
                ))
                .status()
                .expect("job binary copy failed")
        };
    }

    // get the ip_address and port
    // check if a cronframe instance is running
    // send the job to the running cronframe instance
    // localhost::8098/add_cli_job/<expr>/<timeout>/<job>

    let (ip, mut port) = ip_and_port();

    if port_option.is_some() {
        port = port_option.unwrap().parse().unwrap();
    }

    if !is_running(&ip, port) {
        println!("{}", "Error when adding job to CronFrame".red().bold());
        println!("No instance found at: http://{ip}:{port}");
        return;
    }

    let req_url = format!("http://{ip}:{port}/add_cli_job/{escaped_expr}/{timeout}/{job_name}");

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

fn scheduler_command(action: &str, port_option: Option<&String>) {
    let (ip, mut port) = ip_and_port();

    if port_option.is_some() {
        port = port_option.unwrap().parse().unwrap();
    }

    if !is_running(&ip, port) {
        println!("{}", "Scheduler Command error".red().bold());
        println!("No instance found at: http://{ip}:{port}");
        return;
    }

    match action.to_lowercase().as_str() {
        "start" => {
            let req_url = format!("http://{ip}:{port}/start_scheduler");

            match reqwest::blocking::get(req_url) {
                Ok(_) => {
                    println!("Scheduler will soon start.");
                }
                Err(error) => {
                    println!("Error when starting the scheduler");
                    println!("{error}");
                }
            }
        }
        "stop" => {
            let req_url = format!("http://{ip}:{port}/stop_scheduler");

            match reqwest::blocking::get(req_url) {
                Ok(_) => {
                    println!("Scheduler will soon stop.");
                }
                Err(error) => {
                    println!("Error when stopping the scheduler");
                    println!("{error}");
                }
            }
        }
        _ => {
            println!("{}", "Error: scheduler action unknown.".red().bold());
        }
    }
}

fn load_command(file: Option<&String>) {
    let (ip, port) = ip_and_port();
    if !is_running(&ip, port) {
        println!("{}", "Load Command Error".red().bold());
        println!("No instance found at: http://{ip}:{port}");
        return;
    }

    let file_path = match file {
        Some(path) => path.clone(),
        None => format!("{}/.cronframe/job_list.txt", utils::home_dir()),
    };

    match std::fs::read(file_path) {
        Ok(content) => {
            for line in content.lines().into_iter() {
                let line = line.unwrap();
                let cmpt: Vec<_> = line.split(" ").collect();

                let expr = if cmpt.len() == 9 {
                    // expr made of 7 fields
                    format!(
                        "{} {} {} {} {} {} {}",
                        cmpt[0], cmpt[1], cmpt[2], cmpt[3], cmpt[4], cmpt[5], cmpt[6]
                    )
                } else {
                    // expr made of 6 fields (year absent)
                    format!(
                        "{} {} {} {} {} {}",
                        cmpt[0], cmpt[1], cmpt[2], cmpt[3], cmpt[4], cmpt[5]
                    )
                };

                let timeout = if cmpt.len() == 9 { cmpt[7] } else { cmpt[6] };
                let job = if cmpt.len() == 9 { cmpt[8] } else { cmpt[7] };

                add_command(&expr, timeout, job, None);
            }
        }
        Err(err) => {
            println!("{}", err.to_string());
        }
    }
}

fn cronframe_folder() {
    let home_dir = utils::home_dir();

    if !std::path::Path::new(&format!("{home_dir}/.cronframe")).exists() {
        println!("Generating .cronframe directory content...");

        let template_dir = format!("{home_dir}/.cronframe/templates").replace("\\", "/");
        let rocket_toml = format!("[debug]\ntemplate_dir = \"{template_dir}\"\n[release]\ntemplate_dir = \"{template_dir}\"");

        fs::create_dir(format!("{home_dir}/.cronframe"))
            .expect("could not create .cronframe directory");
        fs::create_dir(format!("{home_dir}/.cronframe/cli_jobs"))
            .expect("could not create .cronframe directory");

        web_server::generate_template_dir();

        let _ = fs::write(
            Path::new(&format!("{home_dir}/.cronframe/rocket.toml")),
            rocket_toml,
        );
    }
}

fn is_running(ip: &str, port: u16) -> bool {
    match reqwest::blocking::get(format!("http://{ip}:{port}")) {
        Ok(_) => true,
        Err(_) => false,
    }
}
