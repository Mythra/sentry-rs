extern crate backtrace;
extern crate chrono;
#[macro_use]
extern crate hyper;
extern crate hyper_native_tls;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use chrono::offset::utc::UTC;
use hyper::Client;
use hyper::header::{Headers, ContentType};
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use std::collections::BTreeMap;
use std::env;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::io::BufReader;
use std::io::BufRead;
use std::sync::mpsc::{channel, Receiver, Sender, SendError};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread;

/// The Thread State of the listening Worker that sends items off to sentry.
/// Contains a single atomic boolean for knowing whether or not it's alive cross threads.
struct ThreadState<'a> {
  alive: &'a mut Arc<AtomicBool>,
}
impl<'a> ThreadState<'a> {
  /// Makes the Thread State turn alive.
  fn set_alive(&self) {
    self.alive.store(true, Ordering::Relaxed);
  }
}
impl<'a> Drop for ThreadState<'a> {
  /// "Drops" the Thread State (kills off the thread, and sets itself to not alive).
  fn drop(&mut self) {
    self.alive.store(false, Ordering::Relaxed);
  }
}

/// Implement the worker closure as a trait, incase I ever want to have more than a single worker.
pub trait WorkerClosure<T, P>: Fn(&P, T) -> () + Send + Sync {}
impl<T, F, P> WorkerClosure<T, P> for F where F: Fn(&P, T) -> () + Send + Sync {}

/// A Single Worker thread that sends items to Sentry.
pub struct SingleWorker<T: 'static + Send, P: Clone + Send> {
  parameters: P,
  f: Arc<Box<WorkerClosure<T, P, Output = ()>>>,
  receiver: Arc<Mutex<Receiver<T>>>,
  sender: Mutex<Sender<T>>,
  alive: Arc<AtomicBool>,
}

impl<T: 'static + Debug + Send, P: 'static + Clone + Send> SingleWorker<T, P> {
  /// Creates a new Worker Thread. This realaly should only be used internally, and you
  /// probably shouldn't just go around creating worker threads.
  pub fn new(parameters: P, f: Box<WorkerClosure<T, P, Output = ()>>) -> SingleWorker<T, P> {
    let (sender, reciever) = channel::<T>();

    let worker = SingleWorker {
      parameters: parameters,
      f: Arc::new(f),
      receiver: Arc::new(Mutex::new(reciever)),
      sender: Mutex::new(sender),
      alive: Arc::new(AtomicBool::new(true)),
    };
    SingleWorker::spawn_thread(&worker);
    worker
  }

  /// Internal Method to handle some of the logic of reading from an a AtomicBoolean.
  fn is_alive(&self) -> bool {
    self.alive.clone().load(Ordering::Relaxed)
  }

  /// Spawns the thread for when the worker isn't already working (alive).
  fn spawn_thread(worker: &SingleWorker<T, P>) {
    let mut alive = worker.alive.clone();
    let f = worker.f.clone();
    let receiver = worker.receiver.clone();
    let parameters = worker.parameters.clone();
    thread::spawn(move || {
      let state = ThreadState { alive: &mut alive };
      state.set_alive();

      let lock = match receiver.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
      };

      loop {
        match lock.recv() {
          Ok(value) => f(&parameters, value),
          Err(_) => {
            thread::yield_now();
          }
        };
      }
    });
    while !worker.is_alive() {
      thread::yield_now();
    }
  }

  /// Processes an Event that needs to go to Sentry.
  pub fn work_with(&self, msg: T) -> Result<(), SendError<T>> {
    let alive = self.is_alive();
    if !alive {
      SingleWorker::spawn_thread(self);
    }

    let lock = match self.sender.lock() {
      Ok(guard) => guard,
      Err(poisoned) => poisoned.into_inner(),
    };

    lock.send(msg)
  }
}

#[derive(Clone, Debug, Serialize)]
/// A Stackframe to Send to Sentry.
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
}

#[derive(Clone, Debug, Serialize)]
/// The SDK Representation for Sentry.
pub struct SDK {
  /// The name of the SDK sending the Event.
  pub name: String,
  /// The version of the SDK sending the Event.
  pub version: String,
}

#[derive(Clone, Debug, Serialize)]
/// Information about the device for Sentry.
pub struct Device {
  /// The name of the device.
  pub name: String,
  /// The version of the device.
  pub version: String,
  /// The build of the device.
  pub build: String,
}

#[derive(Clone, Debug)]
/// A Sentry Event.
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
  pub extra: BTreeMap<String, String>,
  /// The fingerprints of this event.
  pub fingerprint: Vec<String>,
}

impl Event {
  /// Turns an event into a string. Due it a special way this way, because renaming a value of a value
  /// inside of serde isn't really friendly, and just feels weird if they made it possible. this method
  /// is super ugly right now, but it works.
  ///
  /// _TODO: Refactor All this_.
  pub fn to_string(&self) -> String {
    let mut base_str = String::new();
    base_str.push_str("{");
    base_str.push_str(&format!("\"event_id\":\"{}\",", self.event_id));
    base_str.push_str(&format!("\"message\":\"{}\",", self.message));
    base_str.push_str(&format!("\"timestamp\":\"{}\",", self.timestamp));
    base_str.push_str(&format!("\"level\": \"{}\",", self.level));
    base_str.push_str(&format!("\"logger\": \"{}\",", self.logger));
    base_str.push_str(&format!("\"platform\": \"{}\",", self.platform));
    base_str.push_str(&format!("\"sdk\": {},",
                               serde_json::to_string_pretty(&self.sdk).unwrap_or("".to_owned())));
    base_str.push_str(&format!("\"device\": {}",
                               serde_json::to_string_pretty(&self.device)
                                 .unwrap_or("".to_owned())));

    if let Some(ref culprit) = self.culprit {
      base_str.push_str(&format!(",\"culprit\": \"{}\"", culprit));
    }
    if let Some(ref server_name) = self.server_name {
      base_str.push_str(&format!(",\"server_name\": \"{}\"", server_name));
    }
    if let Some(ref release) = self.release {
      base_str.push_str(&format!(",\"release\":\"{}\"", release));
    }
    let tag_length = self.tags.len();
    if tag_length > 0 {
      let last_index = tag_length - 1;
      base_str.push_str(",\"tags\": {");
      for (index, item) in self.tags.iter().enumerate() {
        base_str.push_str(&format!("\"{}\":\"{}\"", item.0, item.1));
        if index != last_index {
          base_str.push_str(",");
        }
      }
      base_str.push_str("}");
    }
    if let Some(ref environment) = self.environment {
      base_str.push_str(&format!(",\"environment\": \"{}\"", environment));
    }
    let modules_len = self.modules.len();
    if modules_len > 0 {
      base_str.push_str(",\"modules\": {");
      let last_iter = modules_len - 1;
      for (index, item) in self.modules.iter().enumerate() {
        base_str.push_str(&format!("\"{}\": \"{}\"", item.0, item.1));
        if index != last_iter {
          base_str.push_str(",");
        }
      }
      base_str.push_str("}");
    }
    let extra_len = self.extra.len();
    if extra_len > 0 {
      base_str.push_str(", \"extra\": {");
      let last_iter = extra_len - 1;
      for (index, item) in self.extra.iter().enumerate() {
        base_str.push_str(&format!("\"{}\": \"{}\"", item.0, item.1));
        if index != last_iter {
          base_str.push_str(",");
        }
      }
      base_str.push_str("}");
    }
    if let Some(ref stacktrace) = self.stacktrace {
      base_str.push_str(",\"stacktrace\":{\"frames\": [");
      let max_iter = stacktrace.len() - 1;
      for (index, item) in stacktrace.iter().enumerate() {
        base_str.push_str("{");
        // A Filename comes sometimes prequoted, unless it's nothing. i really have no idea why.
        if item.filename != "" {
          let mut true_filename = item.filename.clone();
          let tf_len = true_filename.len();
          if true_filename.starts_with("\"") {
            true_filename.remove(0);
            true_filename.truncate(tf_len - 1);
          }
          base_str.push_str(&format!("\"filename\": \"{}\",", true_filename.replace("\"", "")));
        } else {
          base_str.push_str(&format!("\"filename\": \"\","));
        }
        base_str.push_str(&format!("\"in_app\": true,"));
        base_str.push_str(&format!("\"function\": \"{}\",", item.function));
        base_str.push_str(&format!("\"lineno\": {},", item.lineno));
        base_str.push_str(&format!("\"pre_context\": {:?},", item.pre_context));
        base_str.push_str(&format!("\"post_context\": {:?},", item.post_context));
        let mut true_context_line = item.context_line.clone();
        let tc_len = true_context_line.len();
        if true_context_line.starts_with("\"") {
          true_context_line.remove(0);
          true_context_line.truncate(tc_len - 1);
        }
        base_str.push_str(&format!("\"context_line\": \"{}\"",
                                   true_context_line.replace("\"", "")));
        base_str.push_str("}");
        if index != max_iter {
          base_str.push_str(",");
        }
      }
      base_str.push_str("]}");
    }
    let fingerprint_len = self.fingerprint.len();
    if fingerprint_len > 0 {
      base_str.push_str(",\"fingerprint\": [");
      let max_iter = fingerprint_len - 1;
      for (index, item) in self.fingerprint.iter().enumerate() {
        base_str.push_str(&format!("\"{}\"", item));
        if index != max_iter {
          base_str.push_str(",");
        }
      }
      base_str.push_str("]");
    }
    base_str.push_str("}");
    base_str
  }
}

impl Event {
  /// A Wrapper around creating a brand new event. May be a little bit of a perf hinderance,
  /// if You have `Strings`, since this method asks for `&str` (and then turns them into Strings).
  /// But if you want to use static strings, or need to pass in one this can be :totes: helpful.
  pub fn new(logger: &str,
             level: &str,
             message: &str,
             culprit: Option<&str>,
             fingerprint: Option<Vec<String>>,
             server_name: Option<&str>,
             stacktrace: Option<Vec<StackFrame>>,
             release: Option<&str>,
             environment: Option<&str>)
             -> Event {

    Event {
      event_id: "".to_owned(),
      message: message.to_owned(),
      timestamp: UTC::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
      level: level.to_owned(),
      logger: logger.to_owned(),
      platform: "other".to_string(),
      sdk: SDK {
        name: "rust-sentry".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
      },
      device: Device {
        name: env::var_os("OSTYPE")
          .and_then(|cs| cs.into_string().ok())
          .unwrap_or("".to_string()),
        version: "".to_string(),
        build: "".to_string(),
      },
      culprit: culprit.map(|c| c.to_owned()),
      server_name: server_name.map(|c| c.to_owned()),
      stacktrace: stacktrace,
      release: release.map(|c| c.to_owned()),
      tags: BTreeMap::new(),
      environment: environment.map(|c| c.to_owned()),
      modules: BTreeMap::new(),
      extra: BTreeMap::new(),
      fingerprint: fingerprint.unwrap_or(vec![]),
    }
  }

  /// Adds a tag to this event.
  pub fn add_tag(&mut self, key: String, value: String) {
    self.tags.insert(key, value);
  }
}

#[derive(Clone, Debug)]
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
pub struct SentryCredentials {
  pub key: String,
  pub secret: String,
  pub host: Option<String>,
  pub project_id: String,
}

/// A Sentry Object, instiates the worker, and actually is what you send your sentry events too.
pub struct Sentry {
  pub server_name: String,
  pub release: String,
  pub environment: String,
  pub worker: Arc<SingleWorker<Event, SentryCredentials>>,
}

header! { (XSentryAuth, "X-Sentry-Auth") => [String] }

impl Sentry {
  /// Creates a new connection to Sentry.
  pub fn new(server_name: String,
             release: String,
             environment: String,
             credentials: SentryCredentials)
             -> Sentry {

    let worker =
      SingleWorker::new(credentials,
                        Box::new(move |credentials, e| { Sentry::post(credentials, &e); }));

    Sentry {
      server_name: server_name,
      release: release,
      environment: environment,
      worker: Arc::new(worker),
    }
  }

  /// Internal method to post a Sentry Message.
  fn post(credentials: &SentryCredentials, e: &Event) {
    info!("Post has been called for Sentry!");
    let mut headers = Headers::new();
    debug!("Created Headers!");
    let timestamp = UTC::now().timestamp().to_string();
    debug!("Got Timestamp for Sentry: [ {:?} ]", timestamp.clone());
    let sentry_auth = format!("Sentry sentry_version=7,sentry_client=rust-sentry/{},\
                               sentry_timestamp={},sentry_key={},sentry_secret={}",
                              env!("CARGO_PKG_VERSION"),
                              timestamp,
                              credentials.key,
                              credentials.secret);
    headers.set(XSentryAuth(sentry_auth));
    headers.set(ContentType::json());
    debug!("Content Headers Set!");

    let body = e.to_string();

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let mut client = Client::with_connector(connector);
    client.set_read_timeout(Some(Duration::new(5, 0)));
    client.set_write_timeout(Some(Duration::new(5, 0)));

    let url = format!("https://{}:{}@{}/api/{}/store/",
                      credentials.key,
                      credentials.secret,
                      credentials.host.clone().unwrap_or("sentry.insops.net".to_owned()),
                      credentials.project_id);

    debug!("Posting body: {:?}", body.clone());

    let res = client.post(&url).headers(headers).body(&body).send();
    if res.is_err() {
      return;
    }
    let mut res = res.unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    info!("Sentry Response: {:?}", body);
  }

  /// Handles a logged event.
  pub fn log_event(&self, e: Event) {
    let _ = self.worker.work_with(e);
  }

  /// Sets up a sentry hook to listen for `panic!()`'s, and post the results to Sentry.
  pub fn register_panic_handler(&self) {
    let none: Option<fn(&std::panic::PanicInfo)> = None;
    self.register_panic_handler_with_func(none);
  }

  pub fn register_panic_handler_with_func<F>(&self, maybe_f: Option<F>)
    where F: Fn(&std::panic::PanicInfo) + 'static + Sync + Send
  {
    info!("Registering Panic Handler for Sentry!");
    let server_name = self.server_name.clone();
    let release = self.release.clone();
    let environment = self.environment.clone();

    let worker = self.worker.clone();

    std::panic::set_hook(Box::new(move |info: &std::panic::PanicInfo| {
      let location = info.location()
        .map(|l| format!("{}: {}", l.file(), l.line()))
        .unwrap_or("Unknown".to_string());
      let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => {
          match info.payload().downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<Any>",
          }
        }
      };

      let mut frames = vec![];
      backtrace::trace(|frame: &backtrace::Frame| {
        backtrace::resolve(frame.ip(), |symbol| {
          let name = symbol.name()
            .map_or("unresolved symbol".to_string(), |name| name.to_string());
          let filename = symbol.filename()
            .map_or("".to_string(), |sym| format!("{:?}", sym));
          let lineno = symbol.lineno().unwrap_or(0);

          let f = File::open(&filename.replace("\"", ""));
          let mut pre_context = Vec::new();
          let mut context_line = String::new();
          let mut post_context = Vec::new();

          if f.is_ok() {
            let file = f.unwrap();
            let buffed_reader = BufReader::new(&file);
            let items = buffed_reader.lines().skip((lineno - 3) as usize).take(5);

            let mut i = 0;
            for item in items {
              if item.is_ok() {
                let true_item = item.unwrap();
                match i {
                  0 | 1 => {
                    pre_context.push(true_item);
                  }
                  2 => {
                    context_line = true_item;
                  }
                  3 | 4 => {
                    post_context.push(true_item);
                  }
                  _ => continue,
                }
              }
              i += 1;
            }
          } else {
            drop(f);
          }

          frames.push(StackFrame {
            filename: filename,
            function: name,
            lineno: lineno,
            pre_context: pre_context,
            post_context: post_context,
            context_line: context_line,
          });
        });

        true
      });

      let event = Event::new("panic",
                             "fatal",
                             msg,
                             Some(&location),
                             None,
                             Some(&server_name),
                             Some(frames),
                             Some(&release),
                             Some(&environment));
      let result = worker.work_with(event.clone());
      if result.is_ok() {
        // Wait for timeout before bailing.
        std::thread::sleep(Duration::from_secs(5));
      }
      if let Some(ref f) = maybe_f {
        f(info);
      }
    }));

    info!("Setup Panic Handler!");
  }

  /// Unregisters the panic handler.
  pub fn unregister_panic_handler(&self) {
    let _ = std::panic::take_hook();
  }

  /// Logs a fatal message to sentry.
  pub fn fatal(&self, logger: &str, message: &str, culprit: Option<&str>) {
    self.log(logger, "fatal", message, culprit, None);
  }

  /// Logs an error message to sentry.
  pub fn error(&self, logger: &str, message: &str, culprit: Option<&str>) {
    self.log(logger, "error", message, culprit, None);
  }

  /// Logs a warning message to sentry.
  pub fn warning(&self, logger: &str, message: &str, culprit: Option<&str>) {
    self.log(logger, "warning", message, culprit, None);
  }

  /// Logs an info message to sentry.
  pub fn info(&self, logger: &str, message: &str, culprit: Option<&str>) {
    self.log(logger, "info", message, culprit, None);
  }

  /// Logs a debug message to sentry.
  pub fn debug(&self, logger: &str, message: &str, culprit: Option<&str>) {
    self.log(logger, "debug", message, culprit, None);
  }

  /// Handles a log call of any level.
  fn log(&self,
         logger: &str,
         level: &str,
         message: &str,
         culprit: Option<&str>,
         fingerprint: Option<Vec<String>>) {

    let fpr = match fingerprint {
      Some(f) => f,
      None => {
        vec![logger.to_string(),
             level.to_string(),
             culprit.map(|c| c.to_string()).unwrap_or("".to_string())]
      }
    };

    let _ = self.worker.work_with(Event::new(logger,
                                             level,
                                             message,
                                             culprit,
                                             Some(fpr),
                                             Some(&self.server_name),
                                             None,
                                             Some(&self.release),
                                             Some(&self.environment)));
  }
}
