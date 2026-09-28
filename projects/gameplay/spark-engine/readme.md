# spark-engine

Engine shell: `SparkRuntime` dual-script frame loop, scenes, mod loading, script domains, and `DataRegistry`.

## Recommended entry (2D games)

```rust
use spark_engine::{NativeGamePlugin, RustPhase, SparkRuntime, run_runtime};
use spark_renderer::WindowConfig;

struct MyPlugin;
impl NativeGamePlugin for MyPlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.add_system_ctx(RustPhase::Update, "sim", |ctx| { /* ... */ });
    }
}

let mut runtime = SparkRuntime::new();
runtime.register_native(&MyPlugin);
run_runtime(WindowConfig { title: "demo".into(), ..Default::default() }, runtime)?;
```

`RuntimeHost2d` implements `WindowPump2d` internally; games **do not** implement the window pump.

## Custom shell (editor / tools)

```rust
use spark_engine::run_window_2d;
use spark_renderer::{WindowConfig, WindowPump2d};

run_window_2d(WindowConfig::default(), my_pump)?;
```

Mod path:

```rust
use spark_engine::{SparkEngine, parse_mod_von};

let mut engine = SparkEngine::new("mods");
let manifest = parse_mod_von(r#"id = "demo""#)?;
```

Window GPU submission is in `spark-renderer-wgpu`. `DataRegistry` stores generic table entries; games interpret their meaning.

```bash
cargo test -p spark-engine
cargo run -p ping-pong
cargo run -p snake
```
