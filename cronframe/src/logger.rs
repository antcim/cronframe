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

pub fn appender_config(log_file: &str) -> log4rs::Config  {
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
    let window_size = 3;
    let roller = FixedWindowRoller::builder()
        .build("log/old_log{}.log", window_size)
        .unwrap();

    let size_limit = 1000 * 1024;

    let trigger = SizeTrigger::new(size_limit);

    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let pattern = "{d(%Y-%m-%d %H:%M:%S %Z)} {l} {t} - {m}{n}";

    let log_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .append(false)
        .build("log/latest.log", Box::new(policy))
        .expect("rolling_logger log file unwrap error");

    let config = Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .build(
            Root::builder()
                .appender("log_file")
                .build(log::LevelFilter::Info),
        )
        .expect("rolling_logger config unwrap error");

    log4rs::init_config(config).expect("rolling_logger init error")
}
