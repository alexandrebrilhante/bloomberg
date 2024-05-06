# blpapi

A Rust wrapper for Bloomberg's `blpapi`.

This is a work in progress and not intended to be used in production in its current version.

## Installation
Download and install the Install [C/C++ BLPAPI](https://www.bloomberg.com/professional/support/api-library/) and set the `BLPAPI_LIB` environment variable to the extract path.

Add the following the following to your `Cargo.toml`:

```toml
[dependencies]
blpapi = { version = "0.1.0", features = [ "derive", "dates" ] }
```

### Example
#### Historical Data

```rust
use blpapi::{RefData, session::{SessionSync, HistOptions}};

#[derive(Default, RefData)]
struct Price {
    px_last: f64,
}

let mut session = SessionSync::new().unwrap();

let securities: &[&str] = &[ "IBM US Equity" ];

let options = HistOptions::new("20240401", "20240430");

let prices = session.hist_data::<_, Price>(securities, options);
```
