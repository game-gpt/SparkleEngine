# ping-pong

纯 Rust 乒乓：`SparkRuntime` + `NativeGamePlugin`，经 `spark_engine::run_runtime` 开窗。

```bash
cargo run -p ping-pong
```

```rust
use ping_pong::runtime::build_runtime;
use spark_engine::run_runtime;

run_runtime(window_config, build_runtime())?;
```

| 内容        | 文件                |
|-------------|---------------------|
| 运行时装配  | `src/runtime.rs`    |
| 球速 / 发球 | `src/ball.rs`       |
| 挡板        | `src/paddle.rs`     |
| 遗留 GameHost | `src/game.rs`（对照） |
| 窗口        | `src/main.rs`       |

无音频、无脚本模组。Studio 可按 Rust 工程打开。
