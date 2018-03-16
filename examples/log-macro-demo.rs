extern crate sentry_rs;
#[macro_use]
extern crate log;

use sentry_rs::models::SentryCredentials;
use sentry_rs::Sentry;
use sentry_rs::logging::SentryLogger;
use std::env;

fn main() {
  let dsn = "https://XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX:YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY@sentry.io/XX";
  let credentials = SentryCredentials::from_str(dsn).unwrap();
  let sentry = Sentry::new(
    "Server Name".to_string(),
    "Release of Your Project Consider using env!()".to_string(),
    "Environment you're deployed in".to_string(),
    credentials,
  );

  SentryLogger::init(sentry, "default logger", log::Level::Warn);
  debug!("This debug message won't be logged to Sentry.");
  info!("This info message won't be logged to Sentry.");
  warn!("This warn message should be logged to Sentry.");
  error!("This error message should be logged to Sentry.");
}
