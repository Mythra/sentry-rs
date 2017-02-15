extern crate sentry_rs;
use sentry_rs::*;
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
      },
      StackFrame {
        filename: "filename.2.stack.frame".to_owned(),
        function: "function.2.stack.frame".to_owned(),
        lineno: 12,
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
fn to_string_shallow_event() {
  let value = generate_shallow_event().to_string();
  assert_eq!(value, r#"{"event_id":"event_id","message":"message","timestamp":"timestamp","level": "level","logger": "logger","platform": "platform","sdk": {
  "name": "sdk_name",
  "version": "sdk_version"
},"device": {
  "name": "device_name",
  "version": "device_version",
  "build": "device_build"
}}"#);
}

#[test]
fn to_string_full_event() {
  let value = generate_full_event().to_string();
  assert_eq!(value, r#"{"event_id":"event_id","message":"message","timestamp":"timestamp","level": "level","logger": "logger","platform": "platform","sdk": {
  "name": "sdk_name",
  "version": "sdk_version"
},"device": {
  "name": "device_name",
  "version": "device_version",
  "build": "device_build"
},"culprit": "culprit","server_name": "server_name","release":"Release","tags": {"tag_key":"tag_value","tag_key_2":"tag_value_2"},"environment": "environment","modules": {"module_key": "module_value","module_key_2": "module_value_2"}, "extra": {"extra_key": "extra_value","extra_key_2": "extra_value_2"},"stacktrace":{"frames": [{"filename": filename.stack.frame,"function": "function.stack.frame","lineno": 10},{"filename": filename.2.stack.frame,"function": "function.2.stack.frame","lineno": 12}]},"fingerprint": ["fingerprint"]}"#);
}
