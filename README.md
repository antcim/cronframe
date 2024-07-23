# CronFrame 0.1.1

This library allows for the definition of cronjobs with macros both on functions in the "global scope" and inside struct types.

# General Information
There are three types of jobs that can be defined:
- global jobs
- functions jobs
- method jobs

Each of these is defined with a macro, a standalone macro for global jobs while function a method jobs require a little bit of setup.

A struct that can host jobs is known as a `cron object` in the context of cronframe and is defined with the cron_obj macro.

Jobs of a cron object must be defined inside a standalone implementation block annotated with the macro cron_impl.

**IMPORTANT:** a cron object must derive the Clone trait

The library supports a daily timeout (timed-out state resets every 24hrs) in ms which is decativated if the value is 0.

During the first run of the library a templates folder will be created in the current directory with 4 files inside it:
- base.html.tera
- index.html.tera
- job.html.tera
- styles.css

By default the server runs on localhost:8098, the port can be changed in the `cronframe.toml` file.

A rolling logger also configurable via `cronframe.toml` provides an archive of 3 files in addition to the latest log.

The default size of a log file is 1MB.

# Defining A Global Job
```rust
#[cron(expr="* * * * * * *", timeout="0")]    
fn hello_job(){
    println!("hello world!");
}

fn main(){
    let cronframe = Cronframe::default();
    cronframe.scheduler();

    // do other stuff or loop to keep it alive...
    loop {
        // sleep here would be nice
    }
}
```

# Defining A Function Job
```rust
#[cron_obj]
#[derive(Clone)] // this trait is required
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
    let cronframe = Cronframe::default();
    cronframe.scheduler();
    // this function collects all functions jobs defined on a cron object
    User::cf_gather_fn(cronframe.clone());

    // do other stuff or loop to keep it alive...
    loop {
        // sleep here would be nice
    }
}
```

# Defining A Method Job
```rust
#[cron_obj]
#[derive(Clone)] // this trait is required
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
    let cronframe = Cronframe::default();

    let mut user1 = User::new_cron_obj(
        "John Smith".to_string(),
        CronFrameExpr::new("0/5", "*", "*", "*", "*", "*", "*", 0)
    );

    // this method collects all jobs defined on a cron object
    user1.cf_gather(cronframe.clone());

    // in alternative if we only wanted to collect method jobs
    // user1.cf_gather_mt(cronframe.clone());

    cronframe.scheduler();

    // do other stuff or loop to keep it alive...
    loop {
        // sleep here would be nice
    }
}
```

# Running Examples
If the example is in a single file like `first.rs` use the following command:
```bash
cargo run --example first
```

If the example is in its own crate like `weather_alert` do the following:
```bash
cd examples/weather_alert
cargo run
```