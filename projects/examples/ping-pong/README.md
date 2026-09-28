# ping-pong

Pure Rust ping-pong: `SparkRuntime` + `NativeGamePlugin`, windowed via `spark_engine::run_runtime`.

```bash
cargo run -p ping-pong
```

```rust
use ping_pong::runtime::build_runtime;
use spark_engine::run_runtime;

run_runtime(window_config, build_runtime())?;
```

| Topic | File |
|-------|------|
| Runtime wiring | `src/runtime.rs` |
| Ball speed / serve | `src/ball.rs` |
| Paddles | `src/paddle.rs` |
| Legacy GameHost | `src/game.rs` (reference) |
| Window | `src/main.rs` |

No audio, no script mods. Studio can open it as a Rust project.
