use std::{sync::LazyLock, time::Instant};

use eframe::egui::mutex::Mutex;

pub struct LogSink {
    pub start: Instant,
    pub entries: Vec<LogEntry>,
}

pub struct LogEntry {
    pub instant: Instant,
    pub level: log::Level,
    pub message: String,
    pub target: String,
}

pub static LOG_SINK: LazyLock<Mutex<LogSink>> = LazyLock::new(|| {
    Mutex::new(LogSink {
        start: Instant::now(),
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
        LOG_SINK.lock().entries.push(LogEntry {
            level: record.level(),
            message: record.args().to_string(),
            target: record.target().to_string(),
            instant: Instant::now(),
        });
    }

    fn flush(&self) {
        LOG_SINK.lock().entries.clear();
    }
}

static LOGGER: Logger = Logger;

/// Initialise the global logger.
pub fn init() -> Result<(), log::SetLoggerError> {
    log::set_max_level(log::LevelFilter::Info);
    log::set_logger(&LOGGER)
}
