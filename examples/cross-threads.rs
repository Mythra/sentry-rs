extern crate sentry_rs;

use sentry_rs::models::SentryCredentials;
use sentry_rs::Sentry;
use std::env;
use std::sync::Arc;
use std::thread;

fn main() {
  let credentials = SentryCredentials {
    scheme: env::var("SENTRY_SCHEME").unwrap_or("https".to_owned()),
    key: env::var("SENTRY_KEY").unwrap_or("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_owned()),
    secret: env::var("SENTRY_SECRET").unwrap_or("YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY".to_owned()),
    host: Some(env::var("SENTRY_HOST").unwrap_or("app.getsentry.com".to_owned())),
    project_id: env::var("SENTRY_PROJECT_ID").unwrap_or("XX".to_owned()),
  };
  let sentry = Arc::new(Sentry::new(
    "Server Name".to_string(),
    "Release of Your Project Consider using env!()".to_string(),
    "Environment you're deployed in".to_string(),
    credentials
  ));

  let other_sentry_one = sentry.clone();
  let other_sentry_two = sentry.clone();

  let thread_one = thread::spawn(move || {
    other_sentry_one.info("thread.one", "Test Message", None, None);
  });
  let thread_two = thread::spawn(move || {
    other_sentry_two.info("thread.two", "Message Test", None, None);
  });

  let _ = thread_one.join();
  let _ = thread_two.join();
}
