# CronFrame

## Defining a Cronjob
To define a cronjob only an annotation is necessary.
Timeout is expressed in ms. (implementation is wip...)
```rust
#[cron(expr = "* * * * * *", timeout = "0")]
fn mycronjob(){
    // do stuff...
}
```

All cronjobs are gathered into a vector of jobs before the main function does anything.

To actually schedule the jobs, an init of the lib is required.

```rust
fn main(){
    CronFrame::init().schedule();
}
```

Here, `init()` collects all the references to our jobs and `schedule()` actually provides their scheduling for execution.

Therefore all function names are gathered automatically.

## How to to run the example
Simply cd to the folder cronrs and run the following:
```bash
cargo run
```
