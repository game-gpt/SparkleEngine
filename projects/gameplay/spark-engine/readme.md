# spark-engine

引擎壳：`SparkRuntime` 双脚本帧循环、场景、模组加载、脚本域与 `DataRegistry`。

## 推荐入口（2D）

```rust
use spark_engine::{NativeGamePlugin, RustPhase, SparkRuntime, run_runtime};
use spark_renderer::WindowConfig;

struct MyPlugin;
impl NativeGamePlugin for MyPlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.add_rust_system_ctx(RustPhase::Update, "sim", |ctx| { /* ... */ });
    }
}

let mut runtime = SparkRuntime::new();
runtime.register_native(&MyPlugin);
run_runtime(WindowConfig { title: "demo".into(), ..Default::default() }, runtime)?;
```

`RuntimeHost2d` 在内部适配窗口泵；游戏**不**实现 `GameHost`。

## 遗留入口（禁止新代码）

`run_game` / `EcsHost2d` / `impl GameHost` 仅兼容旧示例与未迁移游戏，已 `deprecated`。

模组路径：

```rust
use spark_engine::{SparkEngine, parse_mod_von};

let mut engine = SparkEngine::new("mods");
// register_plugin / register_script_system / load_all
let manifest = parse_mod_von(r#"id = "demo""#)?;
```

窗口泵在 `spark-renderer-wgpu`。`DataRegistry` 存通用表项，内容含义由游戏解释。

```bash
cargo test -p spark-engine
cargo run -p ping-pong
cargo run -p snake
```
