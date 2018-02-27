//! Used to export all the possible "Worker" types that can work on a Sentry Queue.
//! Right now there's only the "SingleWorker" which works on it's own single thread.
//! In the future though we might add in something like "MultiWorker" that allows you
//! to work on multiple threads at once.

/// The Trait for a Clojure being able to work on the Sentry Queue of events.
pub trait WorkerClosure<T, P>: Fn(&P, T) -> () + Send + Sync {}
impl<T, F, P> WorkerClosure<T, P> for F
where
  F: Fn(&P, T) -> () + Send + Sync,
{
}

pub mod single;
