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
sentry-rs = "1.4"
```

And then this in your crate root:

```rust
extern crate sentry_rs;
```

## Examples ##

The examples of Rust-Sentry are located inside of the `examples/` directory. The basic usage guide is
you want to create an instance of the `Sentry`, and `SentryCredentials` structs.

## License ##

[Original Rust Sentry (MIT)](https://github.com/aagahi/rust-sentry)

This library is licensed under MIT.
