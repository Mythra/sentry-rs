//! Logging related utilities.

use log::{self, Log, Record, Level, Metadata, SetLoggerError};

use super::Sentry;

/// Logger which implements the `log::Log` trait. This allows logging via the
/// macros defined in the `log` crate.
pub struct SentryLogger {
  // Sentry client used for delivering log messages.
  sentry: Sentry,

  // Name of the logger to log messages with.
  logger_name: String,

  // Minimum level to log messages to Sentry at.
  level: Level,
}

impl SentryLogger {
  /// Construct a new `SentryLogger`.
  ///
  /// # Arguments
  ///
  /// * `sentry` - Sentry client used to deliver log messages.
  /// * `logger_name` - String used as logger name in messages.
  /// * `level` - Minimum level to log messages to Sentry at.
  pub fn new(sentry: Sentry, logger_name: &str, level: Level) -> Self {
    SentryLogger {
      sentry,
      logger_name: logger_name.to_owned(),
      level
    }
  }

  /// Globally initialises a `SentryLogger` as the log facility. This will then be used by the
  /// `log` module's logging macros (e.g. `debug!`, `info!`, etc.).
  ///
  /// # Arguments
  ///
  /// * `sentry` - Sentry client used to deliver log messages.
  /// * `logger_name` - String used as logger name in messages.
  /// * `level` - Minimum level to log messages to Sentry at.
  pub fn init(sentry: Sentry, logger_name: &str, level: Level) -> Result<(), SetLoggerError> {
      log::set_max_level(level.to_level_filter());
      log::set_boxed_logger(Box::new(SentryLogger::new(sentry, logger_name, level)))
  }
}

impl Log for SentryLogger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    metadata.level() <= self.level
  }

  fn log(&self, record: &Record) {
    let metadata = record.metadata();
    if self.enabled(metadata) {
      match metadata.level() {
        Level::Error => self.sentry.error(&self.logger_name, &format!("{}", record.args()), None, None),
        Level::Warn => self.sentry.warning(&self.logger_name, &format!("{}", record.args()), None, None),
        Level::Info => self.sentry.info(&self.logger_name, &format!("{}", record.args()), None, None),
        Level::Debug => self.sentry.debug(&self.logger_name, &format!("{}", record.args()), None, None),
        _ => (), // client doesn't support logging at Trace level
      }
    }
  }

  fn flush(&self) {}
}
