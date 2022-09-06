use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

use crate::error::LogInitError;

static ROOT_LOGGER_NAME: &str = "stdout";
static LOG_LINE_PATTERN: &str = "{d(%Y-%m-%d %H:%M:%S)} | {t} | {({l}):5.5} | {m}{n}";

pub fn init_logger() -> Result<(), LogInitError> {
    let console_appender = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_LINE_PATTERN)))
        .build();
    let appender = Appender::builder().build(ROOT_LOGGER_NAME, Box::new(console_appender));
    let builder = Config::builder().appender(appender);
    let config = builder.build(
        Root::builder()
            .appender(ROOT_LOGGER_NAME)
            .build(log::LevelFilter::Info),
    )?;
    let _handler = log4rs::init_config(config)?;

    Ok(())
}
