# Sentry-Client #

| OS      | Build Status                                                                                                                                                          |
|:--------|:----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Linux   | [![Linux Build Status](https://circleci.com/gh/SecurityInsanity/sentry-rs/tree/master.svg?style=svg)](https://circleci.com/gh/SecurityInsanity/sentry-rs/tree/master) |
| Windows | [![Windows Build status](https://ci.appveyor.com/api/projects/status/yvlgnytb2tir8y4q?svg=true)](https://ci.appveyor.com/project/SecurityInsanity/sentry-rs)          |




Sentry Client is a fork of: [THIS](https://github.com/aagahi/rust-sentry) sentry client,
but contains numerous fixes as well as some code/dependency cleanup.

## Usage ##

Add the following to your rusts `Cargo.toml`:

```toml
[dependencies]
sentry-rs = "1.0"
```

And then this in your crate root:

```rust
extern crate sentry_rs;
```

## Examples ##

If you'd like to simply send a message to sentry you can use the logging interface:
```rust
let credentials = SentryCredentials {
  /// From a Sentry Client Key DSN:
  /// ```text
  /// https://XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX:YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY@ZZZZ/AAA
  /// ```
  ///
  /// The "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" value is your "key".
  /// The "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY" value is your "secret".
  /// The "ZZZZ" value is your "host".
  /// The "AAA" value is your "project_id".
  key: "xx".to_string(),
  secret: "xx".to_string(),
  host: "app.getsentry.com".to_string(),
  project_id: "xx".to_string()
};
let sentry = Sentry::new(
  "Server Name".to_string(),
  "Release Of Your Project Consider using env!()".to_string(),
  "Environment you're deployed in.".to_string(),
  credentials
);
/// Logger Name, Message to Log, Potential Culprit (Option<&str>).
sentry.info("Logger Name", "Message To Log", None);
```

You can use sentry cross threads:
```rust
let sentry = Arc::new(
  Sentry::new(
    "Server Name".to_string(),
    "Release Of Your Project Consider using env!()".to_string(),
    "Environment you're deployed in.".to_string(),
    credentials
  )
);
let sentry1 = sentry.clone();
thread::spawn(move || sentry1.info("test.logger", "Test Message", None));
```

As of Rust v1.10 (and higher), you can use `register_panic_handler()` to automatically
post stack traces on panics:

```
sentry.register_panic_handler();
sentry.unregister_panic_handler();
```

## License ##

[Original Rust Sentry (MIT)](https://github.com/aagahi/rust-sentry)

This library is licensed under MIT.
