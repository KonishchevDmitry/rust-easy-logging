easy-logging
============

An easy way to get logging working in your command line tool. Suitable for simple CLI and prototyping.

Requires a single function call and provides colored logging to stdout/stderr out of the box.

### Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
easy-logging = "*"
```

#### Example:

```rust
use log::{Level, debug, info};

fn main() {
    easy_logging::init(module_path!(), Level::Info).unwrap();
    debug!("Test debug message.");
    info!("Test info message.");
}
```

#### Output with enabled info level:
```
I: Test info message.
```

#### Output with enabled debug level:
```
[22:29:18.084] [   main.rs:006] D: Test debug message.
[22:29:18.085] [   main.rs:007] I: Test info message.
```