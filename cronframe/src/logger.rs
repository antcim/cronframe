use log4rs::{
    append::{
        file::FileAppender,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{self, Appender, Config, Root},
    encode::pattern::PatternEncoder,
};
use rocket::futures::io::Window;

use crate::config::read_config;

pub fn appender_logger(log_file: &str) -> log4rs::Handle {
    let pattern = "{d(%Y-%m-%d %H:%M:%S %Z)} {l} {t} - {m}{n}";

    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .append(false)
        .build(log_file)
        .expect("appender_logger log file unwrap error");

    let config = Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .build(
            Root::builder()
                .appender("log_file")
                .build(log::LevelFilter::Info),
        )
        .expect("appender_logger config unwrap error");

    log4rs::init_config(config).expect("appender_logger init error")
}

pub fn appender_config(log_file: &str) -> log4rs::Config {
    let pattern = "{d(%Y-%m-%d %H:%M:%S %Z)} {l} {t} - {m}{n}";

    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .append(false)
        .build(log_file)
        .expect("appender_config log file unwrap error");

    Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .build(
            Root::builder()
                .appender("log_file")
                .build(log::LevelFilter::Info),
        )
        .expect("appender_logger config unwrap error")
}

pub fn rolling_logger() -> log4rs::Handle {
    let mut window_size = 3;
    let mut size_limit = 1000 * 1024;
    let mut log_dir = "log".to_string();
    let mut latest_file_name = "latest".to_string();
    let mut archive_file_name = "archive".to_string();
    let mut pattern = "{d(%Y-%m-%d %H:%M:%S %Z)} {l} {t} - {m}{n}".to_string();
    let mut level_filter = log::LevelFilter::Info;

    if let Some(config_data) = read_config() {
        if let Some(data) = config_data.logger.archive_files {
            window_size = data;
        }
        if let Some(data) = config_data.logger.file_size {
            size_limit = size_limit * data;
        }
        if let Some(data) = config_data.logger.dir {
            log_dir = data;
        }
        if let Some(data) = config_data.logger.latest_file_name {
            latest_file_name = data;
        }
        if let Some(data) = config_data.logger.archive_file_name {
            archive_file_name = data;
        }
        if let Some(data) = config_data.logger.msg_pattern {
            pattern = data;
        }
        if let Some(data) = config_data.logger.level_filter {
            match data.as_str() {
                "off" => level_filter = log::LevelFilter::Off,
                "error" => level_filter = log::LevelFilter::Error,
                "warn" => level_filter = log::LevelFilter::Warn,
                "debug" => level_filter = log::LevelFilter::Debug,
                _ => (),
            }
        }
    };

    let archive_file = format!("{log_dir}/{archive_file_name}.log").replace(".log", "_{}.log");

    let roller = FixedWindowRoller::builder()
        .build(&archive_file, window_size)
        .unwrap();

    let trigger = SizeTrigger::new(size_limit);

    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let log_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(&pattern)))
        .append(false)
        .build(
            &format!("{log_dir}/{latest_file_name}.log"),
            Box::new(policy),
        )
        .expect("rolling_logger log file unwrap error");

    let config = Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .build(
            Root::builder()
                .appender("log_file")
                .build(level_filter),
        )
        .expect("rolling_logger config unwrap error");

    log4rs::init_config(config).expect("rolling_logger init error")
}
