# CronFrame

## Defining a Cronjob
To define a cronjob on a function only an annotation is necessary.
Timeout is expressed in ms. (implementation is wip...)
```rust
#[cron(expr = "* * * * * *", timeout = "0")]
fn mycronjob(){
    // do stuff...
}
```

To run the jobs, an init is required and the scheduler must be started.

```rust
fn main(){
    CronFrame::default().schedule();
}
```

Here, `default()` collects all the references to our global jobs and `schedule()` actually provides their scheduling for execution.

Refer to cronsrs/main.rs for the scheduling of jobs inside struct types.


## How to to run the example
Simply cd to the folder cronrs and run the following:
```bash
cargo run
```
