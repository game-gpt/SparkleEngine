# spark-engine

引擎壳：固定步帧循环、`EcsHost2d` / `EcsHost3d`、模组加载、脚本域、钩子与 `DataRegistry`。

```rust
use spark_engine::run_game;
use spark_renderer::{GameHost, WindowConfig};

run_game(WindowConfig { title: "demo".into(), ..Default::default() }, my_host)?;
```

也有 `run_game_3d`、`run_app_2d`（`SparkApp` + `SparkPlugin`）、`run_ecs_game`。模组路径：

```rust
use spark_engine::{SparkEngine, parse_mod_von};

let mut engine = SparkEngine::new("mods");
// register_plugin / register_script_system / load_all
// 每帧 begin_frame → 脚本 tick → apply_script_commands
let manifest = parse_mod_von(r#"id = "demo""#)?;
```

窗口泵在 `spark-renderer-wgpu`。`DataRegistry` 存通用表项，内容含义由游戏解释。

```bash
cargo test -p spark-engine
cargo run -p ping-pong
```
