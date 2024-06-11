use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

pub fn default_logger() -> log4rs::Handle {
    let pattern = "{d(%Y-%m-%d %H:%M:%S UTC%Z)} {l} {t} - {m}{n}";

    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .append(false)
        .build("log/cronframe.log")
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