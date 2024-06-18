use log4rs::{
    append::rolling_file::{
        policy::compound::{
            roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
        },
        RollingFileAppender,
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

pub fn default_logger() -> log4rs::Handle {
    let window_size = 3;
    let roller = FixedWindowRoller::builder()
        .build("archive_log/log{}.log", window_size)
        .unwrap();

    let size_limit = 1000 * 1024;

    let trigger = SizeTrigger::new(size_limit);

    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let pattern = "{d(%Y-%m-%d %H:%M:%S UTC%Z)} {l} {t} - {m}{n}";

    let log_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .append(false)
        .build("log/latest.log", Box::new(policy))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .build(
            Root::builder()
                .appender("log_file")
                .build(log::LevelFilter::Info),
        )
        .unwrap();

    log4rs::init_config(config).unwrap()
}
