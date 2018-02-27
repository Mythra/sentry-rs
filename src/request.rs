//! Wraps Hyper 0.11 to provide easier access to requests.
//!
//! Although you're free to use this externally, we really only built it
//! for ourselves.
//!
//! This file is mostly borrowed from [Rusoto](https://github.com/rusoto/rusoto)
//! who is also licensed under MIT, and whose license is available:
//! [HERE](https://github.com/rusoto/rusoto/blob/master/LICENSE)

use futures::{self, Async, Future, Poll, Stream};
use futures::future::{Either, Select2};
use hyper::Client as HyperClient;
use hyper::client::FutureResponse as HyperFutureResponse;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper::Error as HyperError;
use hyper::header::Headers as HyperHeaders;
use hyper::StatusCode;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::{Handle, Timeout};

use std::io::Error as IoError;
use std::error::Error;
use std::fmt;
use std::time::Duration;

/// Stores the response from a HTTP request.
pub struct HttpResponse {
  /// Status code of HTTP Request
  pub status: StatusCode,
  /// Contents of Response
  pub body: Box<Stream<Item = Vec<u8>, Error = HttpDispatchError> + Send>,
  /// Response headers
  pub headers: HyperHeaders,
}

/// Stores the buffered response from a HTTP request.
pub struct BufferedHttpResponse {
  /// Status code of HTTP Request
  pub status: StatusCode,
  /// Contents of Response
  pub body: Vec<u8>,
  /// Response headers
  pub headers: HyperHeaders,
}

/// Future returned from `HttpResponse::buffer`.
pub struct BufferedHttpResponseFuture {
  status: StatusCode,
  headers: HyperHeaders,
  future: futures::stream::Concat2<Box<Stream<Item = Vec<u8>, Error = HttpDispatchError> + Send>>,
}

impl Future for BufferedHttpResponseFuture {
  type Item = BufferedHttpResponse;
  type Error = HttpDispatchError;

  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    self.future.poll().map(|async| {
      async.map(|body| BufferedHttpResponse {
        status: self.status,
        headers: self.headers.clone(),
        body: body,
      })
    })
  }
}

impl HttpResponse {
  /// Buffer the full response body in memory, resulting in a `BufferedHttpResponse`.
  pub fn buffer(self) -> BufferedHttpResponseFuture {
    BufferedHttpResponseFuture {
      status: self.status,
      headers: self.headers,
      future: self.body.concat2(),
    }
  }

  fn from_hyper(hyper_response: HyperResponse) -> HttpResponse {
    HttpResponse {
      status: hyper_response.status(),
      headers: hyper_response.headers().to_owned(),
      body: Box::new(
        hyper_response
          .body()
          .from_err()
          .map(|chunk| chunk.as_ref().to_vec()),
      ),
    }
  }
}

#[derive(Debug, PartialEq)]
/// An error produced when invalid request types are sent.
pub struct HttpDispatchError {
  message: String,
}

impl Error for HttpDispatchError {
  fn description(&self) -> &str {
    &self.message
  }
}

impl fmt::Display for HttpDispatchError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl From<HyperError> for HttpDispatchError {
  fn from(err: HyperError) -> HttpDispatchError {
    HttpDispatchError {
      message: err.description().to_string(),
    }
  }
}

impl From<IoError> for HttpDispatchError {
  fn from(err: IoError) -> HttpDispatchError {
    HttpDispatchError {
      message: err.description().to_string(),
    }
  }
}

#[derive(Debug, PartialEq)]
/// An error produced when the user has an invalid TLS client
pub struct TlsError {
  message: String,
}

impl Error for TlsError {
  fn description(&self) -> &str {
    &self.message
  }
}

impl fmt::Display for TlsError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.message)
  }
}

/// A future that will resolve to an `HttpResponse`.
pub struct HttpClientFuture(ClientFutureInner);

enum ClientFutureInner {
  HyperWithTimeout(Select2<HyperFutureResponse, Timeout>),
  Error(String),
}

impl Future for HttpClientFuture {
  type Item = HttpResponse;
  type Error = HttpDispatchError;

  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    match self.0 {
      ClientFutureInner::Error(ref message) => Err(HttpDispatchError {
        message: message.clone(),
      }),
      ClientFutureInner::HyperWithTimeout(ref mut select_future) => match select_future.poll() {
        Err(Either::A((hyper_err, _))) => Err(hyper_err.into()),
        Err(Either::B((io_err, _))) => Err(io_err.into()),
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(Either::A((hyper_res, _)))) => Ok(Async::Ready(HttpResponse::from_hyper(hyper_res))),
        Ok(Async::Ready(Either::B(((), _)))) => Err(HttpDispatchError {
          message: "Request timed out".into(),
        }),
      },
    }
  }
}

/// A Wrapper around hyper-client for tls connections.
pub struct HttpsClient {
  inner: HyperClient<HttpsConnector<HttpConnector>>,
  handle: Handle,
}

impl HttpsClient {
  /// Create a tls-enabled http client.
  pub fn new(handle: &Handle) -> Result<HttpsClient, TlsError> {
    let connector = match HttpsConnector::new(4, handle) {
      Ok(connector) => connector,
      Err(tls_error) => {
        return Err(TlsError {
          message: format!("Couldn't create NativeTlsClient: {}", tls_error),
        })
      }
    };
    let inner = HyperClient::configure().connector(connector).build(handle);
    Ok(HttpsClient {
      inner: inner,
      handle: handle.clone(),
    })
  }
}

/// A Wrapper around hyper-client for non-tls connections.
pub struct HttpClient {
  inner: HyperClient<HttpConnector>,
  handle: Handle,
}

impl HttpClient {
  /// Create a non-tls-enabled http client.
  pub fn new(handle: &Handle) -> Result<HttpClient, ()> {
    let inner = HyperClient::configure().build(handle);
    Ok(HttpClient {
      inner: inner,
      handle: handle.clone(),
    })
  }
}

/// Trait for implementing HTTP Request/Response
pub trait DispatchRequest {
  /// The future response value.
  type Future: Future<Item = HttpResponse, Error = HttpDispatchError> + 'static;
  /// Dispatch Request, and then return a Response
  fn dispatch(&self, request: HyperRequest, timeout: Option<Duration>) -> Self::Future;
}

impl DispatchRequest for HttpsClient {
  type Future = HttpClientFuture;

  fn dispatch(&self, hyper_request: HyperRequest, timeout: Option<Duration>) -> Self::Future {
    let inner = match Timeout::new(timeout.unwrap_or(Duration::new(5, 0)), &self.handle) {
      Err(err) => ClientFutureInner::Error(format!("Error creating timeout future {}", err)),
      Ok(timeout_future) => {
        let future = self.inner.request(hyper_request).select2(timeout_future);
        ClientFutureInner::HyperWithTimeout(future)
      }
    };

    HttpClientFuture(inner)
  }
}

impl DispatchRequest for HttpClient {
  type Future = HttpClientFuture;

  fn dispatch(&self, hyper_request: HyperRequest, timeout: Option<Duration>) -> Self::Future {
    let inner = match Timeout::new(timeout.unwrap_or(Duration::new(5, 0)), &self.handle) {
      Err(err) => ClientFutureInner::Error(format!("Error creating timeout future {}", err)),
      Ok(timeout_future) => {
        let future = self.inner.request(hyper_request).select2(timeout_future);
        ClientFutureInner::HyperWithTimeout(future)
      }
    };

    HttpClientFuture(inner)
  }
}
