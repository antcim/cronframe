use crate::{config::read_config, utils};
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
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

/// this logger configuration is used for testing
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

/// this is used to change the log file for each new test
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

/// this sets the logger from either the default configuration or from the toml file
pub fn rolling_logger() -> log4rs::Handle {
    let logger_config = read_config().logger;

    let window_size = logger_config.archive_files;
    let size_limit = 1000 * 1024 * logger_config.file_size;
    let mut log_dir = logger_config.dir;
    let latest_file_name = logger_config.latest_file_name;
    let archive_file_name = logger_config.archive_file_name;
    let pattern = logger_config.msg_pattern;
    let level_filter = match logger_config.level_filter.as_str() {
        "off" => log::LevelFilter::Off,
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "debug" => log::LevelFilter::Debug,
        _ => log::LevelFilter::Info,
    };

    if std::env::var("CRONFRAME_CLI").is_ok() {
        let home_dir = utils::home_dir();
        log_dir = format!("{home_dir}/.cronframe/log");
    }

    let archive_file = format!("{log_dir}/{archive_file_name}.log").replace(".log", "_{}.log");

    // retain latest and archive log files at restart as per rolling policy
    if !std::path::Path::new(&format!("{log_dir}/{latest_file_name}")).exists() {
        let _ = std::fs::remove_file(format!(
            "./{log_dir}/{archive_file_name}_{}.log",
            window_size - 1
        ));

        for i in (1..=(window_size - 1)).rev() {
            let _ = std::fs::rename(
                format!("{log_dir}/{archive_file_name}_{}.log", i - 1),
                format!("{log_dir}/{archive_file_name}_{}.log", i),
            );
        }

        let _ = std::fs::rename(
            format!("{log_dir}/{latest_file_name}.log"),
            format!("{log_dir}/{archive_file_name}_0.log"),
        );
    }

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
        .build(Root::builder().appender("log_file").build(level_filter))
        .expect("rolling_logger config unwrap error");

    log4rs::init_config(config).expect("rolling_logger init error")
}
