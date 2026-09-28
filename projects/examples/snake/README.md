# snake

Snake: `SparkRuntime` + `run_runtime`. Arrow keys / WASD to move, R to restart, Esc to quit.

```bash
cargo run -p snake
```

| Topic | Location |
|-------|----------|
| Runtime wiring | `src/runtime.rs` |
| Legacy reference | `SnakeApp` in `src/lib.rs` (`GameHost`, pending removal) |
| Window | `src/main.rs` |

The project directory may carry Valkyrie metadata; this crate is the native play host.
