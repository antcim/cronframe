# CronFrame 0.1.3

CronFrame is both a framework and a CLI tool.

The framework allows for the definition and scheduling of cron jobs with macros both on functions in the "global scope" and inside struct types to be used inside Rust projects.

Job creation without macros is possible, refer to the example in `no_macros.rs` on the [repo](https://github.com/antcim/cronframe).

The CLI tool of the same name is used to spin a global instance of the framework read to accept job that can be added either from cli comands.

## Getting Started
To use the framework in your Rust project:
```sh
$ cargo add cronframe
```

The linkme crate is required for macros to work, more recent versions of linkme might also work.
```sh
$ cargo add linkme@0.3.26
```

To install the CLI tool:
```sh
$ cargo install cronframe
```

## General Information
The cron expression parser used is [cron](https://crates.io/crates/cron).

Scheduling time is in UTC.

There are three types of jobs that can be defined:
- global jobs
- function jobs
- method jobs

Each of these is defined with a macro, a standalone macro for global jobs while function a method jobs require a little bit of setup.

A struct that can host jobs is known as a `cron object` in the context of cronframe and is defined with the `cron_obj` macro.

Jobs of a cron object must be defined inside a standalone implementation block annotated with the macro `cron_impl`.

**NOTICE:** a cron object derives the Clone trait so its fields must too.

The framework supports a daily timeout (timed-out state resets every 24hrs) in ms which is decativated if the value is 0.

During the first run of the framework a templates folder will be created in the current directory with 7 files inside it:
- base.html.tera
- index.htm.tera
- job.html.tera
- tingle.js
- cronframe.js
- styles.css
- tingle.css

By default the server runs on localhost:8098, the port can be changed in the `cronframe.toml` file.

More configuration options available via `cronframe.toml`.

The default size of a log file is 1MB.

## Tutorial
For the tutorial refer to the [website](https://antcim.github.io/cronframe_site/).

## Defining A Global Job
```rust
#[macro_use] 
extern crate cronframe;
use cronframe::CronFrame;

#[cron(expr="* * * * * * *", timeout="0")]    
fn hello_job(){
    println!("hello world!");
}

fn main(){
    // init and gather global cron jobs
    let cronframe = CronFrame::default();
    
    // start the scheduler
    cronframe.start_scheduler();

    // to keep the main thread alive 
    // cronframe.keep_alive();

    // alternatively, start the scheduler and keep main alive
    // cronframe.run();
}
```

## Defining A Function Job
```rust
#[macro_use] 
extern crate cronframe;
use cronframe::CronFrame;
#[cron_obj]
struct User {
    name: String,
}

#[cron_impl]
impl User {
    #[fn_job(expr="* * * * * * *", timeout="0")]    
    fn hello_function_job(){
        println!("hello world!");
    }
}

fn main(){
    let cronframe = CronFrame::default();
    
    // this function collects all function jobs defined on a cron object
    User::cf_gather_fn(cronframe.clone());

    cronframe.start_scheduler();

    // alternatively, start the scheduler and keep main alive
    // cronframe.run();
}
```

## Defining A Method Job
```rust
#[macro_use] 
extern crate cronframe;
use cronframe::{CronFrame, CronFrameExpr};

#[cron_obj]
struct User {
    name: String,
    expr1: CronFrameExpr,
}

#[cron_impl]
impl User {
    #[fn_job(expr="* * * * * * *", timeout="0")]    
    fn hello_function_job(){
        println!("hello world!");
    }

    #[mt_job(expr="expr1")]    
    fn hello_method_job(self){
        println!("hello world!");
    }
}

fn main(){
    let cronframe = CronFrame::default();

    let mut user1 = User::new_cron_obj(
        "John Smith".to_string(),
        CronFrameExpr::new("0/5", "*", "*", "*", "*", "*", "*", 0)
    );

    // this method collects all jobs defined on a cron object
    user1.cf_gather(cronframe.clone());

    // in alternative if we only wanted to collect method jobs
    // user1.cf_gather_mt(cronframe.clone());

    cronframe.start_scheduler();

    // alternatively, start the scheduler and keep main alive
    // cronframe.run();
}
```

## The CLI Tool
Upon first start the tool generates a .cronframe directory inside the user's home directory. 

This directory contains the templates folder, the log folder, the cargo_targets folder and the cli_jobs folder. 

To configure the cronframe instance of the CLI tool with the cronframe.toml file, put it in the .cronframe directory.

## Running Examples
If the example is in a single file like `base_example.rs` use the following command:
```sh
$ cargo run --example base_example
```

If the example is in its own crate like `weather_alert` do the following:
```sh
$ cd examples/weather_alert
$ cargo run
```

## Running Tests
Tests must be run sequentially and not in parallel since they rely on the logger output.
```sh
$ cargo test -- --test-threads=1
```