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
#[macro_use]
extern crate serde_json;
extern crate url;

pub mod models;
pub mod workers;

use chrono::Duration as CDuration;
use chrono::offset::utc::UTC;
use hyper::Client;
use hyper::header::{Headers, ContentType};
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use models::*;
use std::fs::File;
use std::io::Read;
use std::io::BufReader;
use std::io::BufRead;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use workers::single::SingleWorker;

/// The Thread State of the listening Worker that sends items off to sentry.
/// Contains a single atomic boolean for knowing whether or not it's alive cross threads.
pub struct ThreadState<'a> {
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

/// A Sentry Object, instiates the worker, and actually is what you send your sentry events too.
pub struct Sentry {
  pub server_name: String,
  pub release: String,
  pub environment: String,
  pub worker: Arc<SingleWorker<Event, SentryCredentials>>,
  pub reciever: Arc<Mutex<Receiver<String>>>,
}

header! {
  /// A Header representation of X-Sentry-Auth.
  (XSentryAuth, "X-Sentry-Auth") => [String]
}

impl Sentry {
  /// Creates a new connection to Sentry.
  pub fn new(server_name: String, release: String, environment: String, credentials: SentryCredentials) -> Sentry {

    let (the_sender, the_reciever) = channel::<String>();
    let true_sender = Arc::new(Mutex::new(the_sender));
    let worker = SingleWorker::new(credentials,
                                   Box::new(move |credentials, e| {
                                     Sentry::post(credentials, &e);
                                     let _ = true_sender.lock().unwrap().send(e.event_id);
                                   }));

    Sentry {
      server_name: server_name,
      release: release,
      environment: environment,
      worker: Arc::new(worker),
      reciever: Arc::new(Mutex::new(the_reciever)),
    }
  }

  /// Internal method to post a Sentry Message.
  fn post(credentials: &SentryCredentials, e: &Event) {
    info!("Post has been called for Sentry!");
    let mut headers = Headers::new();
    debug!("Created Headers!");
    let timestamp = UTC::now().timestamp().to_string();
    debug!("Got Timestamp for Sentry: [ {:?} ]", timestamp.clone());
    let sentry_auth = format!("Sentry sentry_version=7,sentry_client=sentry-rs/{},\
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
                      credentials.host.clone().unwrap_or("sentry.io".to_owned()),
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

  #[allow(while_true)]
  pub fn register_panic_handler_with_func<F>(&self, maybe_f: Option<F>)
    where F: Fn(&std::panic::PanicInfo) + 'static + Sync + Send
  {
    info!("Registering Panic Handler for Sentry!");
    let server_name = self.server_name.clone();
    let release = self.release.clone();
    let environment = self.environment.clone();

    let worker = self.worker.clone();

    let the_rec = self.reciever.clone();

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

          let mut pre_context = Vec::new();
          let mut context_line = String::new();
          let mut post_context = Vec::new();
          let fixed_filename = filename.replace("\"", "");

          if cfg!(feature = "sourcemap") {
            let f = File::open(&fixed_filename);
            if f.is_ok() {
              let file = f.unwrap();
              let buffed_reader = BufReader::new(&file);
              let items = buffed_reader.lines().skip((lineno - 6) as usize).take(11);

              let mut i = 0;
              for item in items {
                if item.is_ok() {
                  let true_item = item.unwrap();
                  match i {
                    0 | 1 | 2 | 3 | 4 => {
                      pre_context.push(true_item);
                    }
                    5 => {
                      context_line = true_item;
                    }
                    6 | 7 | 8 | 9 | 10 => {
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
          }

          let in_app = !(fixed_filename.starts_with("/buildslave") || fixed_filename == "");

          frames.push(StackFrame {
            filename: filename,
            function: name,
            lineno: lineno,
            pre_context: pre_context,
            post_context: post_context,
            context_line: context_line,
            in_app: in_app,
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
                             Some(&environment),
                             None);
      let recv = the_rec.lock();
      if recv.is_err() {
        info!("Couldn't Grab Recv Mutex, falling back to max timeout...");
        std::thread::sleep(Duration::from_secs(5));
        return;
      }
      let recv = recv.unwrap();
      let event_id = event.event_id.clone();
      let result = worker.work_with(event);
      if result.is_ok() {
        let start_time = UTC::now();
        while true {
          // Wait for sentry before bailing.
          let recived_id = recv.recv_timeout(Duration::from_secs(5));
          if recived_id.is_err() {
            if recived_id.err().unwrap() == RecvTimeoutError::Timeout {
              break;
            }
          } else {
            if recived_id.unwrap() == event_id {
              break;
            }
          }
          if UTC::now().signed_duration_since(start_time) >= CDuration::seconds(5) {
            info!("Didn't recieve event in 5 seconds, bailing anyway.");
            break;
          }
        }
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
  pub fn fatal(&self, logger: &str, message: &str, culprit: Option<&str>, device: Option<Device>) {
    self.log(logger, "fatal", message, culprit, None, device);
  }

  /// Logs an error message to sentry.
  pub fn error(&self, logger: &str, message: &str, culprit: Option<&str>, device: Option<Device>) {
    self.log(logger, "error", message, culprit, None, device);
  }

  /// Logs a warning message to sentry.
  pub fn warning(&self, logger: &str, message: &str, culprit: Option<&str>, device: Option<Device>) {
    self.log(logger, "warning", message, culprit, None, device);
  }

  /// Logs an info message to sentry.
  pub fn info(&self, logger: &str, message: &str, culprit: Option<&str>, device: Option<Device>) {
    self.log(logger, "info", message, culprit, None, device);
  }

  /// Logs a debug message to sentry.
  pub fn debug(&self, logger: &str, message: &str, culprit: Option<&str>, device: Option<Device>) {
    self.log(logger, "debug", message, culprit, None, device);
  }

  /// Handles a log call of any level.
  fn log(&self,
         logger: &str,
         level: &str,
         message: &str,
         culprit: Option<&str>,
         fingerprint: Option<Vec<String>>,
         device: Option<Device>) {

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
                                             Some(&self.environment),
                                             device));
  }
}
