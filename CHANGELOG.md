## 1.4.0 (March 2nd, 2017)

- Make programs not wait the full 5 second HTTP Timeout on panic.

## 1.3.0 (March 2nd, 2017)

- Made Context Lines an optional feature that are enabled by default.
- Bumped amount of context lines from 2 on each side to 5.
- Cleaned up Serialization of Events. To no longer be a manual spaghetti.
- Bumped Backtrace-rs/Chrono dependencies to their latest version.

## 1.2.0 (Feburary 28th, 2017)

- Added the ability to have a callback function on your panic handler.

## 1.1.0 (Feburary 18th, 2017)

- Add 3 context lines always if we can parse the file.
- Added some Cargo.toml attributes.

## 1.0.0 (Feburary 15th, 2017)

- Initial Release of Sentry-RS.