extern crate env_logger;
extern crate sentry_rs;

use sentry_rs::models::SentryCredentials;
use sentry_rs::Sentry;
use std::{env, thread};

fn main() {
  env_logger::init();
  let credentials = SentryCredentials {
    scheme: env::var("SENTRY_SCHEME").unwrap_or("https".to_owned()),
    key: env::var("SENTRY_KEY").unwrap_or("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_owned()),
    secret: env::var("SENTRY_SECRET").unwrap_or("YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY".to_owned()),
    host: Some(env::var("SENTRY_HOST").unwrap_or("app.getsentry.com".to_owned())),
    project_id: env::var("SENTRY_PROJECT_ID").unwrap_or("XX".to_owned()),
  };
  let sentry = Sentry::new(
    "Test Boxen".to_string(),
    "0.1.0".to_string(),
    "Production".to_string(),
    credentials,
  );

  sentry.register_panic_handler();
  let t1 = thread::spawn(|| {
    panic!("Panic Handler Testing");
  });
  let _ = t1.join();
  sentry.unregister_panic_handler();
}
