//! Provides a list of "Models" for Sentry. These actually represent what gets sent to sentry,
//! and tries to follow their guidelines.
//!
//! We don't include any attributes that we ourselves don't use. It may be worthwhile one day to actually
//! include some of these when it's worthwhile for downstream consumers.

use chrono::offset::utc::UTC;
use serde_json::{to_string, Value};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::str::FromStr;
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// A Stackframe to Send to Sentry. Each attribute is described in detail [HERE].
///
/// [HERE]: https://docs.sentry.io/clientdev/attributes/
pub struct StackFrame {
  /// The Filename that this StackFrame originated from.
  pub filename: String,
  /// The function this stackframe originated from.
  pub function: String,
  /// The line number this stackframe originated from.
  pub lineno: u32,
  /// The lines that come before it for context.
  pub pre_context: Vec<String>,
  /// The lines that come after the error line for context.
  pub post_context: Vec<String>,
  /// The line that through the error for context.
  pub context_line: String,
  /// Whether or not this error orginates "inside the app". E.g. not parts of rust itself.
  /// For us we use this as anything not in /buildslave/ + main
  pub in_app: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// The SDK Representation for Sentry. Each attribute is described in detail [HERE].
///
/// [HERE]: https://docs.sentry.io/clientdev/attributes/
pub struct SDK {
  /// The name of the SDK sending the Event.
  pub name: String,
  /// The version of the SDK sending the Event.
  pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Information about the device for Sentry. Each attribute is described in detail [HERE].
///
/// [HERE]: https://docs.sentry.io/clientdev/attributes/
pub struct Device {
  /// The name of the device.
  pub name: String,
  /// The version of the device.
  pub version: String,
  /// The build of the device.
  pub build: String,
}

#[derive(Clone, Debug, PartialEq)]
/// An Event that gets sent to Sentry. Each attribute is described in detail [HERE].
///
/// [HERE]: https://docs.sentry.io/clientdev/attributes/
pub struct Event {
  /// The event id of this event.
  pub event_id: String,
  /// The message of this event.
  pub message: String,
  /// The timestamp of this event.
  pub timestamp: String,
  /// The level of warning for this event.
  pub level: String,
  /// The logger for this event.
  pub logger: String,
  /// The platform for this event.
  pub platform: String,
  /// The SDK of this event.
  pub sdk: SDK,
  /// The Device of this event.
  pub device: Device,
  /// The culprit of this event.
  pub culprit: Option<String>,
  /// The server name for this event.
  pub server_name: Option<String>,
  /// The stacktrace of this event.
  pub stacktrace: Option<Vec<StackFrame>>,
  /// The release of this event.
  pub release: Option<String>,
  /// The tags of this event.
  pub tags: BTreeMap<String, String>,
  /// The environment this event occured in.
  pub environment: Option<String>,
  /// The modules of this event.
  pub modules: BTreeMap<String, String>,
  /// The extra info for this event.
  pub extra: HashMap<String, Value>,
  /// The fingerprints of this event.
  pub fingerprint: Vec<String>,
}

/// "Prepares" a string for being encoded to json. Right now this only strips off strings that start/end
/// with " since it seems to barf on being sent, but in the future it could do more things.
pub fn prep_string(to_prep: &str) -> String {
  let mut to_return = to_prep.to_owned();
  if to_prep != "" {
    if to_prep.starts_with("\"") {
      let tlen = to_return.len();
      to_return.remove(0);
      to_return.truncate(tlen - 2);
    }
  }
  to_return
}

impl Event {
  /// Serializes an Event for Sentry. This is implemented in a custom way,
  /// because renaming the value of a field to a key/value pair in serde_json
  /// was something I couldn't figure out how to do, and would probably be uglier
  /// than manually building. Maybe not. Anyway we are manually building the json object.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use sentry_rs::models::Event;
  /// let event: Event = Event::new("my logger", "INFO", "a message", Some("jerk"),
  /// Some(vec!["fingerprint".to_owned()]), Some("server name"), Some(vec![]),
  /// Some("release"), Some("production"), None);
  ///
  /// let as_string: String = event.to_string();
  /// println!("{}", as_string);
  /// ```
  pub fn to_string(&self) -> String {
    let mut value: Value = json!({
      "event_id": self.event_id,
      "message": self.message,
      "timestamp": self.timestamp,
      "level": self.level,
      "logger": self.logger,
      "platform": self.platform,
      "sdk": json!(self.sdk),
      "device": json!(self.device),
      "culprit": json!(self.culprit),
      "server_name": json!(self.server_name),
      "release": json!(self.release),
    });
    let tag_length = self.tags.len();
    if tag_length > 0 {
      value["tags"] = json!(self.tags);
    }
    if let Some(ref environment) = self.environment {
      value["environment"] = json!(environment);
    }
    let modules_len = self.modules.len();
    if modules_len > 0 {
      value["modules"] = json!(self.modules);
    }
    let extra_len = self.extra.len();
    if extra_len > 0 {
      value["extra"] = json!(self.extra);
    }
    if let Some(ref stacktrace) = self.stacktrace {
      let frames = stacktrace
        .iter()
        .map(|item| json!(item))
        .collect::<Vec<Value>>();
      value["stacktrace"] = json!({
        "frames": json!(frames),
      });
    }
    let fingerprint_len = self.fingerprint.len();
    if fingerprint_len > 0 {
      value["fingerprint"] = json!(self.fingerprint);
    }

    to_string(&value).unwrap()
  }
}

impl Event {
  /// A Wrapper around creating a brand new event. May be a little bit of a perf hinderance,
  /// if You have `Strings`, since this method asks for `&str` (and then turns them into Strings).
  /// But if you want to use static strings, or need to pass in one this can be :totes: helpful.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use sentry_rs::models::Event;
  /// let event: Event = Event::new("my logger", "PANIC", "my message", None, None, None, None, None, None, None);
  /// ```
  ///
  /// ```rust
  /// use sentry_rs::models::Event;
  /// let event: Event = Event::new("my logger", "INFO", "a message", Some("jerk"),
  /// Some(vec!["fingerprint".to_owned()]), Some("server name"), Some(vec![]),
  /// Some("release"), Some("production"), None);
  /// ```
  pub fn new(
    logger: &str,
    level: &str,
    message: &str,
    culprit: Option<&str>,
    fingerprint: Option<Vec<String>>,
    server_name: Option<&str>,
    stacktrace: Option<Vec<StackFrame>>,
    release: Option<&str>,
    environment: Option<&str>,
    device: Option<Device>,
  ) -> Event {

    Event {
      event_id: "".to_owned(),
      message: message.to_owned(),
      timestamp: UTC::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
      level: level.to_owned(),
      logger: logger.to_owned(),
      platform: "other".to_string(),
      sdk: SDK {
        name: "sentry-rs".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
      },
      device: device.unwrap_or(Device {
        name: env::var_os("OSTYPE")
          .and_then(|cs| cs.into_string().ok())
          .unwrap_or("".to_string()),
        version: "".to_string(),
        build: "".to_string(),
      }),
      culprit: culprit.map(|c| c.to_owned()),
      server_name: server_name.map(|c| c.to_owned()),
      stacktrace: stacktrace,
      release: release.map(|c| c.to_owned()),
      tags: BTreeMap::new(),
      environment: environment.map(|c| c.to_owned()),
      modules: BTreeMap::new(),
      extra: HashMap::new(),
      fingerprint: fingerprint.unwrap_or(vec![]),
    }
  }

  /// Adds a tag to this event. Useful for when you're trying to add a specific piece of context.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use sentry_rs::models::Event;
  /// let mut event: Event = Event::new("my logger", "PANIC", "my message", None, None, None, None, None, None, None);
  /// event.add_tag("User".to_owned(), "Chris Pratt".to_owned());
  /// ```
  pub fn add_tag(&mut self, key: String, value: String) {
    self.tags.insert(key, value);
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Some Sentry Credentials. Which although not immediatly obvious are super easy to get.
/// Firsrt things first, go fetch your Client Keys (DSN) like you normally would for a project.
/// Should look something like:
///
/// ```text
/// https://XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX:YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY@ZZZZ/AAA
/// ```
///
/// The "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" value is your "key".
/// The "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY" value is your "secret".
/// The "ZZZZ" value is your "host".
/// The "AAA" value is your "project_id".
///
/// # Examples
///
/// ```rust
/// extern crate sentry_rs;
/// use sentry_rs::models::SentryCredentials;
/// use std::env;
///
/// fn main() {
///   let credentials = SentryCredentials {
///     key: env::var("SENTRY_KEY").unwrap_or("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_owned()),
///     secret: env::var("SENTRY_SECRET").unwrap_or("YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY".to_owned()),
///     host: Some(env::var("SENTRY_HOST").unwrap_or("sentry.io".to_owned())),
///     project_id: env::var("SENTRY_PROJECT_ID").unwrap_or("XX".to_owned()),
///   };
/// }
/// ```
///
/// You can also parse sentry credentials from a String:
///
/// ```rust
/// extern crate sentry_rs;
/// use sentry_rs::models::*;
///
/// fn main() {
///   let sentry_creds: SentryCredentials =
///     "https://XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX:YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY@ZZZZ/AAA"
///     .to_owned()
///     .parse::<SentryCredentials>().unwrap();
/// }
/// ```
pub struct SentryCredentials {
  pub key: String,
  pub secret: String,
  pub host: Option<String>,
  pub project_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialsParseError {
  BadUrl,
  NoApiKey,
  NoApiSecret,
  NoHostname,
  BadProjectId,
  NoProjectId,
}

impl FromStr for SentryCredentials {
  type Err = CredentialsParseError;

  fn from_str(to_parse: &str) -> Result<SentryCredentials, CredentialsParseError> {
    let attempt_parse = Url::parse(to_parse);
    if attempt_parse.is_err() {
      return Err(CredentialsParseError::BadUrl);
    }
    let parsed = attempt_parse.unwrap();
    let potential_username = parsed.username();
    if potential_username.is_empty() {
      // The "Username" is equal to the API Key for Sentry Credentials.
      return Err(CredentialsParseError::NoApiKey);
    }
    let potential_password = parsed.password();
    if potential_password.is_none() {
      // The "password" is equal to the API Secret for Sentry Credentials.
      return Err(CredentialsParseError::NoApiSecret);
    }
    let potential_hostname = parsed.host_str();
    if potential_hostname.is_none() {
      return Err(CredentialsParseError::NoHostname);
    }
    let potential_project_id = parsed.path_segments().and_then(|paths| paths.last());
    if potential_project_id.is_none() {
      return Err(CredentialsParseError::BadProjectId);
    }
    let project_id = potential_project_id.unwrap();
    if project_id.is_empty() {
      return Err(CredentialsParseError::NoProjectId);
    }
    Ok(SentryCredentials {
      key: potential_username.to_owned(),
      secret: potential_password.unwrap().to_owned(),
      host: Some(potential_hostname.unwrap().to_owned()),
      project_id: project_id.to_owned(),
    })
  }
}
