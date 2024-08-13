use cronframe::{config::read_config, utils, web_server, CronFilter, CronFrame};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use clap::{arg, command};

fn main() {
    std::env::set_var("CRONFRAME_CLI", "true");

    // cli args parsing
    let matches = command!()
        .version("0.0.1")
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
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }
}

fn start_command() {
    cronframe_folder();
    let _build = Command::new("cronframe")
        .args(["run"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("cronframe run failed");

    let (ip_address, port) = ip_and_port();
    println!("CronFrame will soon be available at: http://{ip_address}:{port}");
}

fn shutdown_command() {
    let (ip_address, port) = ip_and_port();
    let req_url = format!("http://{ip_address}:{port}/shutdown");

    match reqwest::blocking::get(req_url) {
        Ok(_) => {
            println!("CronFrame will soon shutdown.");
        }
        Err(error) => {
            println!("Error:");
            println!("{error}");
        }
    }
}

fn run_command() {
    cronframe_folder();
    let _ = CronFrame::init(Some(CronFilter::CLI), true).run();
}

fn add_command(expr: &str, timeout: &str, job: &str, port_option: Option<&String>) {
    let home_dir = utils::home_dir();

    let escaped_expr = expr.replace("/", "slh");

    let tmp: Vec<_> = job.split("/").collect();
    let job_name = tmp.last().unwrap().replace(".rs", "");

    println!("Compiling {job_name} JOB");

    if Path::new(&job).is_file() {
        // compile the "script" job
        let tmp: Vec<_> = job.split("/").collect();
        let job_name = tmp.last().unwrap().replace(".rs", "");

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

        let _ = Command::new("cp")
            .args([
                &job_name,
                &format!("{home_dir}/.cronframe/cli_jobs/{job_name}"),
            ])
            .current_dir(format!(
                "{home_dir}/.cronframe/cargo_targets/{job_name}/release"
            ))
            .status()
            .expect("job compilation failed");
    }

    // send the job to the running cronframe instance
    // localhost::8098/add_cli_job/<expr>/<timeout>/<job>

    let (ip_address, mut port) = ip_and_port();

    if port_option.is_some() {
        port = port_option.unwrap().parse().unwrap();
        println!("PORT OPTION = {port}");
    }

    let req_url =
        format!("http://{ip_address}:{port}/add_cli_job/{escaped_expr}/{timeout}/{job_name}");

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

fn cronframe_folder() {
    let home_dir = utils::home_dir();

    if !std::path::Path::new(&format!("{home_dir}/.cronframe")).exists() {
        println!("Generating .cronframe directory content...");

        let template_dir = format!("{home_dir}/.cronframe/templates");
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

fn ip_and_port() -> (String, u16) {
    match read_config() {
        Some(config_data) => {
            if let Some(webserver_data) = config_data.webserver {
                (
                    webserver_data.ip.unwrap_or_else(|| "127.0.0.1".to_string()),
                    webserver_data.port.unwrap_or_else(|| 8098),
                )
            } else {
                ("localhost".to_string(), 8098)
            }
        }
        None => ("localhost".to_string(), 8098),
    }
}
