# CronFrame

## Defining a Cronjob
To define a cronjob only an annotation is necessary.
```rust
#[cron("* * * * * *")]
fn mycronjob(){

}
```

To setup the schedule of such jobs we need to provide the auxiliary functions to the library.

```rust
fn main(){
    CronFrame::init()
        .schedule(vec![mycronjob_aux_1, mycronjob_aux_2])
        .start();
}
```

Here, `schedule()` collects all the references to our jobs and `start()` actually provides their scheduling for execution.

Therefore all function names are not gathered automatically.

## How to to run the example
Simply cd to the folder cronrs and run the following:
```bash
cargo run
```