use crate::cronjob::CronJob;
use chrono::Duration;
use cron::Schedule;
use std::any::Any;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum JobBuilder<'a> {
    Global {
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    },
    Method {
        name: &'a str,
        job: fn(arg: Arc<Box<dyn Any + Send + Sync>>),
        cron_expr: String,
        timeout: String,
        instance: Arc<Box<dyn Any + Send + Sync>>,
    },
    Function {
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    },
    CLI {
        name: &'a str,
        cron_expr: &'a str,
        timeout: &'a str,
    },
}

impl<'a> JobBuilder<'a> {
    pub const fn global_job(
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    ) -> Self {
        JobBuilder::Global {
            name,
            job,
            cron_expr,
            timeout,
        }
    }

    pub const fn method_job(
        name: &'a str,
        job: fn(Arc<Box<dyn Any + Send + Sync>>),
        cron_expr: String,
        timeout: String,
        instance: Arc<Box<dyn Any + Send + Sync>>,
    ) -> Self {
        JobBuilder::Method {
            name,
            job,
            cron_expr,
            timeout,
            instance,
        }
    }

    pub const fn function_job(
        name: &'a str,
        job: fn(),
        cron_expr: &'a str,
        timeout: &'a str,
    ) -> Self {
        JobBuilder::Function {
            name,
            job,
            cron_expr,
            timeout,
        }
    }

    pub const fn cli_job(name: &'a str, cron_expr: &'a str, timeout: &'a str) -> Self {
        JobBuilder::CLI {
            name,
            cron_expr,
            timeout,
        }
    }

    // it matches on the job variant to build and builds it
    pub fn build(self) -> CronJob {
        match self {
            Self::Global {
                name,
                job,
                cron_expr,
                timeout,
            } => {
                let schedule =
                    Schedule::from_str(cron_expr).expect("Failed to parse cron expression!");
                let timeout: i64 = timeout.parse().expect("Failed to parse timeout!");
                let timeout = if timeout > 0 {
                    Some(Duration::milliseconds(timeout))
                } else {
                    None
                };
                CronJob::new_global(name, job, schedule, timeout)
            }
            Self::Method {
                name,
                job,
                cron_expr,
                timeout,
                instance,
            } => {
                let schedule =
                    Schedule::from_str(&cron_expr).expect("Failed to parse cron expression!");
                let timeout: i64 = timeout.parse().expect("Failed to parse timeout!");
                let timeout = if timeout > 0 {
                    Some(Duration::milliseconds(timeout))
                } else {
                    None
                };

                CronJob::new_method(name, instance, job, schedule, timeout)
            }
            Self::Function {
                name,
                job,
                cron_expr,
                timeout,
            } => {
                let schedule =
                    Schedule::from_str(cron_expr).expect("Failed to parse cron expression!");
                let timeout: i64 = timeout.parse().expect("Failed to parse timeout!");
                let timeout = if timeout > 0 {
                    Some(Duration::milliseconds(timeout))
                } else {
                    None
                };

                CronJob::new_function(name, job, schedule, timeout)
            }
            Self::CLI {
                name,
                cron_expr,
                timeout,
            } => {
                let cron_expr = cron_expr.replace("slh", "/").replace("%20", " ");
                let schedule =
                    Schedule::from_str(&cron_expr).expect("Failed to parse cron expression!");
                let timeout: i64 = timeout.parse().expect("Failed to parse timeout!");
                let timeout = if timeout > 0 {
                    Some(Duration::milliseconds(timeout))
                } else {
                    None
                };

                CronJob::new_cli(name, schedule, timeout)
            }
        }
    }
}
