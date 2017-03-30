extern crate sentry_rs;

use sentry_rs::models::SentryCredentials;
use sentry_rs::Sentry;
use std::env;

fn main() {
  let credentials = SentryCredentials {
    key: env::var("SENTRY_KEY").unwrap_or("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_owned()),
    secret: env::var("SENTRY_SECRET").unwrap_or("YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY".to_owned()),
    host: Some(env::var("SENTRY_HOST").unwrap_or("app.getsentry.com".to_owned())),
    project_id: env::var("SENTRY_PROJECT_ID").unwrap_or("XX".to_owned()),
  };
  let sentry = Sentry::new(
    "Server Name".to_string(),
    "Release of Your Project Consider using env!()".to_string(),
    "Environment you're deployed in".to_string(),
    credentials
  );

  // Logger Name, Message to Log, Potential Culprit (Option<&str>), Device (Option<Device>).
  sentry.info("Logger Name", "Message To Log", None, None);
}