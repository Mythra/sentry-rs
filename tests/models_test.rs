extern crate sentry_rs;

use sentry_rs::models::*;
use std::collections::BTreeMap;

pub fn generate_shallow_event() -> Event {
  Event {
    event_id: "event_id".to_owned(),
    message: "message".to_owned(),
    timestamp: "timestamp".to_owned(),
    level: "level".to_owned(),
    logger: "logger".to_owned(),
    platform: "platform".to_owned(),
    sdk: SDK {
      name: "sdk_name".to_owned(),
      version: "sdk_version".to_owned(),
    },
    device: Device {
      name: "device_name".to_owned(),
      version: "device_version".to_owned(),
      build: "device_build".to_owned(),
    },
    culprit: None,
    server_name: None,
    stacktrace: None,
    release: None,
    tags: BTreeMap::new(),
    environment: None,
    modules: BTreeMap::new(),
    extra: BTreeMap::new(),
    fingerprint: vec![],
  }
}

pub fn generate_full_event() -> Event {
  let mut tags = BTreeMap::new();
  tags.insert("tag_key".to_owned(), "tag_value".to_owned());
  tags.insert("tag_key_2".to_owned(), "tag_value_2".to_owned());
  let mut modules = BTreeMap::new();
  modules.insert("module_key".to_owned(), "module_value".to_owned());
  modules.insert("module_key_2".to_owned(), "module_value_2".to_owned());
  let mut extras = BTreeMap::new();
  extras.insert("extra_key".to_owned(), "extra_value".to_owned());
  extras.insert("extra_key_2".to_owned(), "extra_value_2".to_owned());
  Event {
    event_id: "event_id".to_owned(),
    message: "message".to_owned(),
    timestamp: "timestamp".to_owned(),
    level: "level".to_owned(),
    logger: "logger".to_owned(),
    platform: "platform".to_owned(),
    sdk: SDK {
      name: "sdk_name".to_owned(),
      version: "sdk_version".to_owned(),
    },
    device: Device {
      name: "device_name".to_owned(),
      version: "device_version".to_owned(),
      build: "device_build".to_owned(),
    },
    culprit: Some("culprit".to_owned()),
    server_name: Some("server_name".to_owned()),
    stacktrace: Some(vec![
      StackFrame {
        filename: "filename.stack.frame".to_owned(),
        function: "function.stack.frame".to_owned(),
        lineno: 10,
        pre_context: vec![
          "filename: \"filename.stack.frame\".to_owned()".to_owned(),
          "function: \"function.stack.frame\".to_owned()".to_owned()
        ],
        context_line: "context_line: \"context_line\"".to_owned(),
        post_context: vec![
          "filename: \"filename.stack.frame\".to_owned()".to_owned(),
          "function: \"function.stack.frame\".to_owned()".to_owned()
        ],
        in_app: true
      },
      StackFrame {
        filename: "filename.2.stack.frame".to_owned(),
        function: "function.2.stack.frame".to_owned(),
        lineno: 12,
        pre_context: Vec::new(),
        context_line: "".to_owned(),
        post_context: Vec::new(),
        in_app: false
      },
    ]),
    release: Some("Release".to_owned()),
    tags: tags,
    environment: Some("environment".to_owned()),
    modules: modules,
    extra: extras,
    fingerprint: vec![
      "fingerprint".to_owned()
    ],
  }
}

#[test]
pub fn to_string_shallow_event() {
  let value = generate_shallow_event().to_string();
  assert_eq!(value, r#"{"culprit":null,"device":{"build":"device_build","name":"device_name","version":"device_version"},"event_id":"event_id","level":"level","logger":"logger","message":"message","platform":"platform","release":null,"sdk":{"name":"sdk_name","version":"sdk_version"},"server_name":null,"timestamp":"timestamp"}"#);
}

#[test]
pub fn to_string_full_event() {
  let value = generate_full_event().to_string();
  assert_eq!(value, r#"{"culprit":"culprit","device":{"build":"device_build","name":"device_name","version":"device_version"},"environment":"environment","event_id":"event_id","extra":{"extra_key":"extra_value","extra_key_2":"extra_value_2"},"fingerprint":["fingerprint"],"level":"level","logger":"logger","message":"message","modules":{"module_key":"module_value","module_key_2":"module_value_2"},"platform":"platform","release":"Release","sdk":{"name":"sdk_name","version":"sdk_version"},"server_name":"server_name","stacktrace":{"frames":[{"context_line":"context_line: \"context_line\"","filename":"filename.stack.frame","function":"function.stack.frame","in_app":true,"lineno":10,"post_context":["filename: \"filename.stack.frame\".to_owned()","function: \"function.stack.frame\".to_owned()"],"pre_context":["filename: \"filename.stack.frame\".to_owned()","function: \"function.stack.frame\".to_owned()"]},{"context_line":"","filename":"filename.2.stack.frame","function":"function.2.stack.frame","in_app":false,"lineno":12,"post_context":[],"pre_context":[]}]},"tags":{"tag_key":"tag_value","tag_key_2":"tag_value_2"},"timestamp":"timestamp"}"#);
}

#[test]
pub fn prep_string_cuts_off_string_in_quotes() {
  let test_string = "\"\"";
  let finalized_string = prep_string(test_string);

  assert_eq!(finalized_string, "");
}

#[test]
pub fn test_sentry_creds_parsing() {
  let test_string = "https://XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX:YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY@ZZZZ/AAA"
    .to_owned()
    .parse::<SentryCredentials>();
  assert!(test_string.is_ok());
  let manual_creation = SentryCredentials {
    key: "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_owned(),
    secret: "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY".to_owned(),
    host: Some("zzzz".to_owned()),
    project_id: "AAA".to_owned()
  };
  assert_eq!(test_string.unwrap(), manual_creation);
}

#[test]
pub fn test_sentry_creds_parsing_failure() {
  let first_test_string = "https://sentry.io/aaa"
    .to_owned()
    .parse::<SentryCredentials>();
  let second_test_string = "https://aaaaaa@sentry.io/aaa"
    .to_owned()
    .parse::<SentryCredentials>();
  let third_test_string = "https://aaa:bbb@sentry.io/"
    .to_owned()
    .parse::<SentryCredentials>();

  assert!(first_test_string.is_err());
  assert!(second_test_string.is_err());
  assert!(third_test_string.is_err());
}