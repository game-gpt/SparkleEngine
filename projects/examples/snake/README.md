# snake

贪吃蛇：`SparkRuntime` + `run_runtime`。方向键 / WASD 移动，R 重开，Esc 退出。

```bash
cargo run -p snake
```

| 内容       | 位置              |
|------------|-------------------|
| 运行时装配 | `src/runtime.rs`  |
| 遗留对照   | `src/lib.rs` 中 `SnakeApp`（`GameHost`，待删） |
| 窗口       | `src/main.rs`     |

工程目录可带 Valkyrie 元数据；本 crate 是 native 试玩宿主。
