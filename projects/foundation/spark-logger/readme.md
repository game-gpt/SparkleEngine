# spark-logger

Leveled logging with pluggable sinks. Production paths use structured `LogEvent` / `EventId`.

```rust
use spark_logger::{Logger, install_global};

let logger = Logger::builder().build();
install_global(logger);
```

Also `install_std(path, min_level)`, `StderrSink`, `FileSink`, and test helper `MemorySink`. `global()` returns the installed instance. Format-string `Logger::log` is a raw bypass for human reading, not a program protocol.

```bash
cargo test -p spark-logger
```
