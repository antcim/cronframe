# CronFrame 0.1.0

This library allows for the definition of cronjobs with macros both on functions in the "global scope" and inside struct types.

# General Information
There are three types of jobs that can be defined:
- global jobs
- functions jobs
- method jobs

Each of these is defined with a macro, a standalone macro for global jobs while function a method jobs require a little bit of setup.

As struct that can host jobs is known as a cron object in the context of cronframe and is defined with the cron_obj macro.

Jobs of a cron object must be defined inside a standalone implementation block annotated with the macro cron_impl.

**IMPORTANT:** a cron object must derive the Clone trait

The library supports a daily timeout in ms which is decativated if the value is 0.

# Defining A Global Job
```rust
#[cron(expr="* * * * * * *", timeout="0")]    
fn hello_job(){
    println!("hello world!");
}

fn main(){
    let cronframe = Cronframe::default();
    cronframe.scheduler();
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
    fn hello_method_job(){
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
}
```