use log4rs::{
    append::{console::ConsoleAppender, file::FileAppender},
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

use crate::error::LogInitError;

static LOG_LINE_PATTERN: &str = "{d(%Y-%m-%d %H:%M:%S)} | {t} | {({l}):5.5} | {m}{n}";

fn console_appender() -> Appender {
    let console_appender = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_LINE_PATTERN)))
        .build();
    Appender::builder().build("console", Box::new(console_appender))
}

fn file_appender() -> Result<Appender, LogInitError> {
    let appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_LINE_PATTERN)))
        .build("/server/logs/admin.log")?;

    Ok(Appender::builder().build("file", Box::new(appender)))
}

pub fn init_logger() -> Result<(), LogInitError> {
    let root_appender = console_appender();
    let file_appender = file_appender()?;
    let builder = Config::builder()
        .appender(root_appender)
        .appender(file_appender);
    let config = builder.build(
        Root::builder()
            .appender("console")
            .appender("file")
            .build(log::LevelFilter::Info),
    )?;
    let _handler = log4rs::init_config(config)?;

    Ok(())
}
