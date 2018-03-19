extern crate sentry_rs;
#[macro_use]
extern crate log;

use sentry_rs::models::SentryCredentials;
use sentry_rs::Sentry;
use sentry_rs::logging::SentryLogger;
use std::env;

fn main() {
  let credentials = SentryCredentials {
    scheme: env::var("SENTRY_SCHEME").unwrap_or("https".to_owned()),
    key: env::var("SENTRY_KEY").unwrap_or("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_owned()),
    secret: env::var("SENTRY_SECRET").unwrap_or("YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY".to_owned()),
    host: Some(env::var("SENTRY_HOST").unwrap_or("app.getsentry.com".to_owned())),
    project_id: env::var("SENTRY_PROJECT_ID").unwrap_or("XX".to_owned()),
  };
  let sentry = Sentry::new(
    "Server Name".to_string(),
    "Release of Your Project Consider using env!()".to_string(),
    "Environment you're deployed in".to_string(),
    credentials,
  );

  SentryLogger::init(sentry, "default logger".to_string(), log::Level::Warn);
  debug!("This debug message won't be logged to Sentry.");
  info!("This info message won't be logged to Sentry.");
  warn!("This warn message should be logged to Sentry.");
  error!("This error message should be logged to Sentry.");
}
