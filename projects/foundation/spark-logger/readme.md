# spark-logger

分级日志与可插拔 sink。正式路径使用结构化 `LogEvent` / `EventId`。

```rust
use spark_logger::{Logger, install_global};

let logger = Logger::builder().build();
install_global(logger);
```

也提供 `install_std(path, min_level)`、`StderrSink`、`FileSink`，以及单测用的 `MemorySink`。`global()` 取已安装实例。格式串式
`Logger::log` 是 raw 旁路，给人口读，不当程序协议。

```bash
cargo test -p spark-logger
```
