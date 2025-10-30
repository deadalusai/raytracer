use std::sync::{LazyLock, Mutex};

use time_format::TimeStampMs;

pub struct LogSink {
    pub entries: Vec<LogEntry>,
}

pub struct LogEntry {
    pub time: TimeStampMs,
    pub level: log::Level,
    pub message: String,
    pub target: String,
}

pub static LOG_SINK: LazyLock<Mutex<LogSink>> = LazyLock::new(|| {
    Mutex::new(LogSink {
        entries: vec![],
    })
});

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let entry = LogEntry {
            time: time_format::now_ms().expect("Before 1970?"),
            level: record.level(),
            message: record.args().to_string(),
            target: record.target().to_string(),
        };
        LOG_SINK.lock().unwrap().entries.push(entry);
    }

    fn flush(&self) {
    }
}

static LOGGER: Logger = Logger;

/// Initialise the global logger.
pub fn init() -> Result<(), log::SetLoggerError> {
    log::set_max_level(log::LevelFilter::Info);
    log::set_logger(&LOGGER)
}
