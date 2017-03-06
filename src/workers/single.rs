//! Houses the implementation of anything for the "SingleWorker",
//! Which is the single threaded worker for sentry.

use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, SendError};
use std::thread;
use ::ThreadState;
use ::workers::WorkerClosure;

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